// Render de uma JANELA geográfica do COG em resolução nativa (2 m), Rust puro.
// lon/lat -> UTM via fórmula fechada (sem PROJ); leitura só dos tiles da janela.
// Índices de pixel são derivados e clampados às dimensões lidas do próprio COG;
// casts f64<->usize são inerentes ao mapeamento geo->pixel.
#![allow(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]

use std::io::Cursor;

use image::{ExtendedColorType, ImageEncoder};
use reqwest::blocking::Client;
use tiff::decoder::{Decoder, DecodingResult};
use tiff::tags::Tag;

use super::cog_overview::{HttpRangeReader, proto};
use crate::application::ports::ProviderError;

/// Forward Transverse Mercator (WGS84) — lon/lat (graus) -> UTM (E,N) metros.
fn lonlat_to_utm(lon_deg: f64, lat_deg: f64, zone: u8, south: bool) -> (f64, f64) {
    let a = 6_378_137.0_f64;
    let f = 1.0 / 298.257_223_563;
    let e2 = f * (2.0 - f);
    let ep2 = e2 / (1.0 - e2);
    let k0 = 0.9996;

    let lat = lat_deg.to_radians();
    let lon = lon_deg.to_radians();
    let lon0 = (f64::from(zone) * 6.0 - 183.0).to_radians();

    let n = a / (1.0 - e2 * lat.sin().powi(2)).sqrt();
    let t = lat.tan().powi(2);
    let c = ep2 * lat.cos().powi(2);
    let aa = lat.cos() * (lon - lon0);

    let m = a
        * ((1.0 - e2 / 4.0 - 3.0 * e2 * e2 / 64.0 - 5.0 * e2.powi(3) / 256.0) * lat
            - (3.0 * e2 / 8.0 + 3.0 * e2 * e2 / 32.0 + 45.0 * e2.powi(3) / 1024.0)
                * (2.0 * lat).sin()
            + (15.0 * e2 * e2 / 256.0 + 45.0 * e2.powi(3) / 1024.0) * (4.0 * lat).sin()
            - (35.0 * e2.powi(3) / 3072.0) * (6.0 * lat).sin());

    let easting = k0
        * n
        * (aa
            + (1.0 - t + c) * aa.powi(3) / 6.0
            + (5.0 - 18.0 * t + t * t + 72.0 * c - 58.0 * ep2) * aa.powi(5) / 120.0)
        + 500_000.0;
    let mut northing = k0
        * (m + n
            * lat.tan()
            * (aa * aa / 2.0
                + (5.0 - t + 9.0 * c + 4.0 * c * c) * aa.powi(4) / 24.0
                + (61.0 - 58.0 * t + t * t + 600.0 * c - 330.0 * ep2) * aa.powi(6) / 720.0));
    if south {
        northing += 10_000_000.0;
    }
    (easting, northing)
}

/// EPSG UTM WGS84 -> (zona, hemisfério sul). 326zz=Norte, 327zz=Sul.
fn epsg_to_utm_zone(epsg: u32) -> Option<(u8, bool)> {
    match epsg {
        32601..=32660 => Some(((epsg - 32600) as u8, false)),
        32701..=32760 => Some(((epsg - 32700) as u8, true)),
        _ => None,
    }
}

struct GeoRef {
    epsg: u32,
    x0: f64,
    y0: f64,
    sx: f64,
    sy: f64,
}

fn read_georef(decoder: &mut Decoder<HttpRangeReader>) -> Result<GeoRef, ProviderError> {
    let scale = decoder
        .get_tag_f64_vec(Tag::ModelPixelScaleTag)
        .map_err(proto)?;
    let tie = decoder
        .get_tag_f64_vec(Tag::ModelTiepointTag)
        .map_err(proto)?;
    let keys = decoder
        .get_tag_u16_vec(Tag::GeoKeyDirectoryTag)
        .map_err(proto)?;
    if scale.len() < 2 || tie.len() < 6 {
        return Err(ProviderError::Protocol("missing geotransform".to_owned()));
    }
    let (sx, sy) = (scale[0], scale[1]);
    // tiepoint (i,j,k) -> (X,Y,Z); origem do raster
    let (i, j) = (tie[0], tie[1]);
    let (x, y) = (tie[3], tie[4]);
    let x0 = x - i * sx;
    let y0 = y + j * sy;

    // ProjectedCSTypeGeoKey = 3072, TIFFTagLocation 0 (valor inline)
    let mut epsg = 0u32;
    if keys.len() >= 4 {
        let num = keys[3] as usize;
        for n in 0..num {
            let b = 4 + n * 4;
            if b + 3 < keys.len() && keys[b] == 3072 && keys[b + 1] == 0 {
                epsg = u32::from(keys[b + 3]);
            }
        }
    }
    if epsg == 0 {
        return Err(ProviderError::Protocol(
            "no projected EPSG in COG".to_owned(),
        ));
    }
    Ok(GeoRef {
        epsg,
        x0,
        y0,
        sx,
        sy,
    })
}

/// Renderiza a janela `bbox` (lon/lat: `[min_lon,min_lat,max_lon,max_lat]`) do COG
/// em resolução nativa (ou o overview mais próximo de `target` px), em PNG.
///
/// IO bloqueante — chame dentro de `spawn_blocking`.
pub fn render_window_png(
    client: &Client,
    url: &str,
    bbox: [f64; 4],
    target: u32,
) -> Result<Vec<u8>, ProviderError> {
    let reader = HttpRangeReader::new(client.clone(), url.to_owned())?;
    let mut decoder = Decoder::new(reader).map_err(proto)?;
    let geo = read_georef(&mut decoder)?;
    let (full_w, full_h) = decoder.dimensions().map_err(proto)?;

    let (zone, south) = epsg_to_utm_zone(geo.epsg)
        .ok_or_else(|| ProviderError::Protocol(format!("unsupported CRS EPSG:{}", geo.epsg)))?;

    // 4 cantos lon/lat -> UTM, pega o retângulo alinhado a eixo em UTM.
    let corners = [
        (bbox[0], bbox[1]),
        (bbox[2], bbox[1]),
        (bbox[2], bbox[3]),
        (bbox[0], bbox[3]),
    ];
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for (lon, lat) in corners {
        let (e, north) = lonlat_to_utm(lon, lat, zone, south);
        min_x = min_x.min(e);
        max_x = max_x.max(e);
        min_y = min_y.min(north);
        max_y = max_y.max(north);
    }

    // UTM -> pixel full-res. row cresce pra sul (y diminui).
    let to_px = |x: f64, y: f64| ((x - geo.x0) / geo.sx, (geo.y0 - y) / geo.sy);
    let (c_a, r_a) = to_px(min_x, max_y); // canto superior-esquerdo
    let (c_b, r_b) = to_px(max_x, min_y); // inferior-direito
    let fc0 = c_a.min(c_b).floor().max(0.0) as i64;
    let fr0 = r_a.min(r_b).floor().max(0.0) as i64;
    let fc1 = c_a.max(c_b).ceil().min(f64::from(full_w)) as i64;
    let fr1 = r_a.max(r_b).ceil().min(f64::from(full_h)) as i64;
    if fc1 <= fc0 || fr1 <= fr0 {
        return Err(ProviderError::Protocol("window outside scene".to_owned()));
    }

    // Escolhe nível de overview tal que a janela caiba em ~target px.
    let span = (fc1 - fc0).max(fr1 - fr0) as f64;
    let mut level = 0u32;
    while (span / f64::from(1u32 << level)) > f64::from(target) {
        level += 1;
    }

    // Navega até o IFD do nível; limita ao número real de overviews.
    for _ in 0..level {
        if !decoder.more_images() {
            break;
        }
        decoder.next_image().map_err(proto)?;
    }
    let (lvl_w, lvl_h) = decoder.dimensions().map_err(proto)?;
    let scale = f64::from(full_w) / f64::from(lvl_w); // ~2^level efetivo

    // Janela em pixels do nível.
    let lc0 = ((fc0 as f64) / scale).floor().max(0.0) as u32;
    let lr0 = ((fr0 as f64) / scale).floor().max(0.0) as u32;
    let lc1 = (((fc1 as f64) / scale).ceil() as u32).min(lvl_w);
    let lr1 = (((fr1 as f64) / scale).ceil() as u32).min(lvl_h);
    let out_w = (lc1 - lc0).max(1);
    let out_h = (lr1 - lr0).max(1);

    let mut out = vec![0u8; (out_w as usize) * (out_h as usize) * 3];

    let (tw, th) = decoder.chunk_dimensions();
    let tiles_across = lvl_w.div_ceil(tw).max(1);

    let tcol0 = lc0 / tw;
    let tcol1 = (lc1 - 1) / tw;
    let trow0 = lr0 / th;
    let trow1 = (lr1 - 1) / th;

    for trow in trow0..=trow1 {
        for tcol in tcol0..=tcol1 {
            let index = (trow * tiles_across + tcol) as usize;
            let (cw, ch) = decoder.chunk_data_dimensions(index as u32);
            let DecodingResult::U8(buf) = decoder.read_chunk(index as u32).map_err(proto)? else {
                return Err(ProviderError::Protocol("unexpected chunk type".to_owned()));
            };
            let tile_x = tcol * tw;
            let tile_y = trow * th;
            for ty in 0..ch {
                let gy = tile_y + ty;
                if gy < lr0 || gy >= lr1 {
                    continue;
                }
                let oy = (gy - lr0) as usize;
                for tx in 0..cw {
                    let gx = tile_x + tx;
                    if gx < lc0 || gx >= lc1 {
                        continue;
                    }
                    let ox = (gx - lc0) as usize;
                    let si = ((ty * cw + tx) as usize) * 3;
                    let di = (oy * out_w as usize + ox) * 3;
                    if si + 3 <= buf.len() && di + 3 <= out.len() {
                        out[di] = buf[si];
                        out[di + 1] = buf[si + 1];
                        out[di + 2] = buf[si + 2];
                    }
                }
            }
        }
    }

    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(Cursor::new(&mut png))
        .write_image(&out, out_w, out_h, ExtendedColorType::Rgb8)
        .map_err(proto)?;
    Ok(png)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utm_central_meridian_origin() {
        // No meridiano central (lon0 = -69 p/ zona 19), lat 0 -> E=500000, N=0 (norte).
        let (e, n) = lonlat_to_utm(-69.0, 0.0, 19, false);
        assert!((e - 500_000.0).abs() < 1.0, "e={e}");
        assert!(n.abs() < 1.0, "n={n}");
    }

    #[test]
    fn utm_south_offset() {
        let (_e, n) = lonlat_to_utm(-69.0, 0.0, 19, true);
        assert!((n - 10_000_000.0).abs() < 1.0, "n={n}");
    }

    #[test]
    fn utm_known_point_sao_paulo_z23s() {
        // Ponto em SP (~ -46.633, -23.55), zona 23S. Confere ordem de grandeza.
        let (e, n) = lonlat_to_utm(-46.633, -23.55, 23, true);
        assert!((330_000.0..345_000.0).contains(&e), "e={e}");
        assert!((7_390_000.0..7_410_000.0).contains(&n), "n={n}");
    }

    #[test]
    fn epsg_mapping() {
        assert_eq!(epsg_to_utm_zone(32719), Some((19, true)));
        assert_eq!(epsg_to_utm_zone(32623), Some((23, false)));
        assert_eq!(epsg_to_utm_zone(4326), None);
    }
}
