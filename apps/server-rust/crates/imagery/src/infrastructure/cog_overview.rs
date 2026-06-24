// Leitura de overview de COG (BigTIFF Deflate/RGB) sobre HTTP Range, em Rust puro.
// Casts entre u64/i64/usize são inerentes a offsets de bytes deste leitor.
#![allow(
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::io::{self, Cursor, Read, Seek, SeekFrom};

use image::{ExtendedColorType, ImageEncoder};
use reqwest::blocking::Client;
use tiff::ColorType;
use tiff::decoder::{Decoder, DecodingResult};

use crate::application::ports::ProviderError;

const CHUNK: u64 = 256 * 1024;

/// `Read + Seek` sobre arquivo remoto via HTTP Range, com buffer para reduzir
/// requisições (a crate `tiff` faz muitos seeks/reads pequenos).
pub(crate) struct HttpRangeReader {
    client: Client,
    url: String,
    pos: u64,
    len: u64,
    buf: Vec<u8>,
    buf_start: u64,
}

impl HttpRangeReader {
    pub(crate) fn new(client: Client, url: String) -> Result<Self, ProviderError> {
        // GET de 1 byte para obter o tamanho total via Content-Range.
        let resp = client
            .get(&url)
            .header(reqwest::header::RANGE, "bytes=0-0")
            .send()
            .map_err(|e| map_err(&e))?;
        if !resp.status().is_success() {
            return Err(ProviderError::Unavailable);
        }
        let len = resp
            .headers()
            .get(reqwest::header::CONTENT_RANGE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.rsplit('/').next().map(str::to_owned))
            .and_then(|v| v.parse::<u64>().ok())
            .ok_or_else(|| ProviderError::Protocol("missing content-range total".to_owned()))?;
        Ok(Self {
            client,
            url,
            pos: 0,
            len,
            buf: Vec::new(),
            buf_start: 0,
        })
    }

    fn ensure(&mut self, pos: u64, want: usize) -> io::Result<()> {
        let end = self.buf_start + self.buf.len() as u64;
        if !self.buf.is_empty() && pos >= self.buf_start && pos + want as u64 <= end {
            return Ok(());
        }
        let fetch = CHUNK.max(want as u64);
        let last = (pos + fetch - 1).min(self.len.saturating_sub(1));
        let resp = self
            .client
            .get(&self.url)
            .header(reqwest::header::RANGE, format!("bytes={pos}-{last}"))
            .send()
            .map_err(io::Error::other)?;
        if !resp.status().is_success() {
            return Err(io::Error::other("range request failed"));
        }
        self.buf = resp.bytes().map_err(io::Error::other)?.to_vec();
        self.buf_start = pos;
        Ok(())
    }
}

impl Read for HttpRangeReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.len {
            return Ok(0);
        }
        let want = out.len().min((self.len - self.pos) as usize);
        if want == 0 {
            return Ok(0);
        }
        self.ensure(self.pos, want)?;
        let off = (self.pos - self.buf_start) as usize;
        let src = self.buf.get(off..).unwrap_or(&[]);
        let n = want.min(src.len());
        let chunk = src.get(..n).unwrap_or(&[]);
        let dst = out
            .get_mut(..n)
            .ok_or_else(|| io::Error::other("short buffer"))?;
        dst.copy_from_slice(chunk);
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for HttpRangeReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new = match pos {
            SeekFrom::Start(p) => p as i64,
            SeekFrom::End(p) => self.len as i64 + p,
            SeekFrom::Current(p) => self.pos as i64 + p,
        };
        if new < 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "negative seek"));
        }
        self.pos = new as u64;
        Ok(self.pos)
    }
}

fn map_err(e: &reqwest::Error) -> ProviderError {
    if e.is_timeout() {
        ProviderError::Timeout
    } else {
        ProviderError::Unavailable
    }
}

/// Renderiza um overview do COG em PNG.
///
/// Escolhe o maior overview cuja maior dimensão seja `<= max_size` (mais nítido
/// sem gerar PNG gigante); se nenhum couber, usa o menor disponível.
///
/// Faz IO bloqueante — chame dentro de `spawn_blocking`.
pub fn render_overview_png(
    client: Client,
    url: &str,
    max_size: u32,
) -> Result<Vec<u8>, ProviderError> {
    // 1ª passada: levanta as dimensões de cada IFD (full-res + overviews).
    let dims = collect_dims(client.clone(), url)?;
    let target = pick_overview(&dims, max_size);

    // 2ª passada: navega até o IFD escolhido e lê a imagem.
    let reader = HttpRangeReader::new(client, url.to_owned())?;
    let mut decoder = Decoder::new(reader).map_err(proto)?;
    for _ in 0..target {
        decoder.next_image().map_err(proto)?;
    }

    let (w, h) = decoder.dimensions().map_err(proto)?;
    match decoder.colortype().map_err(proto)? {
        ColorType::RGB(8) => {}
        other => {
            return Err(ProviderError::Protocol(format!(
                "unsupported color type: {other:?}"
            )));
        }
    }
    let DecodingResult::U8(rgb) = decoder.read_image().map_err(proto)? else {
        return Err(ProviderError::Protocol("unexpected bit depth".to_owned()));
    };

    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(Cursor::new(&mut png))
        .write_image(&rgb, w, h, ExtendedColorType::Rgb8)
        .map_err(proto)?;
    Ok(png)
}

fn collect_dims(client: Client, url: &str) -> Result<Vec<u32>, ProviderError> {
    let reader = HttpRangeReader::new(client, url.to_owned())?;
    let mut decoder = Decoder::new(reader).map_err(proto)?;
    let mut dims = Vec::new();
    loop {
        let (w, h) = decoder.dimensions().map_err(proto)?;
        dims.push(w.max(h));
        if decoder.more_images() {
            decoder.next_image().map_err(proto)?;
        } else {
            break;
        }
    }
    Ok(dims)
}

/// Índice do IFD: maior dimensão `<= max_size`; se nenhum, o menor (último).
fn pick_overview(dims: &[u32], max_size: u32) -> usize {
    let mut best: Option<(usize, u32)> = None;
    for (i, &d) in dims.iter().enumerate() {
        if d <= max_size && best.is_none_or(|(_, bd)| d > bd) {
            best = Some((i, d));
        }
    }
    best.map_or_else(
        || {
            dims.iter()
                .enumerate()
                .min_by_key(|(_, d)| **d)
                .map_or(0, |(i, _)| i)
        },
        |(i, _)| i,
    )
}

pub(crate) fn proto<E: std::fmt::Display>(e: E) -> ProviderError {
    ProviderError::Protocol(e.to_string())
}
