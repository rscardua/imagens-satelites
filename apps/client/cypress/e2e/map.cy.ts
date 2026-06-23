/// <reference types="cypress" />

// E2E do mapa (US1–US3). Requer backend e frontend no ar (quickstart.md).
describe('Mapa de imagens CBERS-4A', () => {
  it('carrega o mapa e a barra de ferramentas', () => {
    cy.visit('/')
    cy.get('.maplibregl-canvas', { timeout: 10000 }).should('be.visible')
    cy.get('.toolbar').should('exist')
    cy.contains('INPE').should('be.visible')
    cy.contains('NASA').should('be.visible')
  })

  it('alterna para a fonte NASA (camada de tiles)', () => {
    cy.visit('/')
    cy.contains('NASA').click()
    cy.get('.maplibregl-canvas', { timeout: 10000 }).should('be.visible')
  })
})
