import { describe, it, expect } from 'vitest'
import { readFileSync } from 'fs'
import { resolve, dirname } from 'path'
import { fileURLToPath } from 'url'

const __dirname = dirname(fileURLToPath(import.meta.url))

describe('Global Styles', () => {
  const mainTsPath = resolve(__dirname, '../main.ts')
  const globalScssPath = resolve(__dirname, '../styles/global.scss')
  const mainTs = readFileSync(mainTsPath, 'utf-8')
  const globalScss = readFileSync(globalScssPath, 'utf-8')

  it('main.ts imports global.scss', () => {
    expect(mainTs).toContain("import './styles/global.scss'")
  })

  it('Google Fonts load (Space Grotesk, JetBrains Mono)', () => {
    expect(globalScss).toContain('Space+Grotesk')
    expect(globalScss).toContain('JetBrains+Mono')
    expect(globalScss).toContain('fonts.googleapis')
  })

  it('Global CSS reset applied', () => {
    expect(globalScss).toContain('box-sizing: border-box')
    expect(globalScss).toContain('margin: 0')
    expect(globalScss).toContain('padding: 0')
  })

  it('Body has correct background and text color', () => {
    expect(globalScss).toContain('--bg-primary')
    expect(globalScss).toContain('--text-primary')
    expect(globalScss).toContain('background: var(--bg-primary)')
    expect(globalScss).toContain('color: var(--text-primary)')
  })

  it('Grid overlay displays on body', () => {
    expect(globalScss).toContain('body::before')
    expect(globalScss).toContain('position: fixed')
    expect(globalScss).toContain('linear-gradient')
    expect(globalScss).toContain('pointer-events: none')
  })

  it('Selection uses accent color', () => {
    expect(globalScss).toContain('::selection')
    expect(globalScss).toContain('--accent')
  })

  it('Smooth scroll enabled', () => {
    expect(globalScss).toContain('scroll-behavior: smooth')
  })

  it('Body uses Space Grotesk font', () => {
    expect(globalScss).toContain('--font-display')
    expect(globalScss).toContain('font-family: var(--font-display)')
  })

  it('Code elements use JetBrains Mono font', () => {
    expect(globalScss).toContain('--font-mono')
    expect(globalScss).toContain('font-family: var(--font-mono)')
  })

  it('Link colors use accent', () => {
    expect(globalScss).toContain('--accent')
  })
})