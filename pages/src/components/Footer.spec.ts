import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import Footer from './Footer.vue'

describe('Footer', () => {
  it('renders footer with top border', () => {
    const wrapper = mount(Footer)
    const footer = wrapper.find('.footer')
    expect(footer.exists()).toBe(true)
    expect(footer.classes()).toContain('footer')
  })

  it('renders logo with mark', () => {
    const wrapper = mount(Footer)
    const logo = wrapper.find('.footer-logo')
    expect(logo.exists()).toBe(true)
    expect(logo.text()).toContain('>')
  })

  it('renders logo text "simple_ssh"', () => {
    const wrapper = mount(Footer)
    const logo = wrapper.find('.footer-logo')
    expect(logo.text()).toContain('simple_ssh')
  })

  it('renders "MIT Licensed" text', () => {
    const wrapper = mount(Footer)
    const license = wrapper.find('.footer-license')
    expect(license.exists()).toBe(true)
    expect(license.text()).toBe('MIT Licensed')
  })

  it('renders three links: GitHub, Docs, Crates.io', () => {
    const wrapper = mount(Footer)
    const links = wrapper.findAll('.footer-link')

    expect(links.length).toBe(3)
    expect(links[0].text()).toBe('GitHub')
    expect(links[1].text()).toBe('Docs')
    expect(links[2].text()).toBe('Crates.io')
  })

  it('GitHub link has correct href', () => {
    const wrapper = mount(Footer)
    const githubLink = wrapper.findAll('.footer-link')[0]
    expect(githubLink.attributes('href')).toBe('https://github.com/juliankahlert/simple_ssh')
    expect(githubLink.attributes('target')).toBe('_blank')
  })

  it('Docs link has correct href', () => {
    const wrapper = mount(Footer)
    const docsLink = wrapper.findAll('.footer-link')[1]
    expect(docsLink.attributes('href')).toBe('https://docs.rs/simple_ssh')
    expect(docsLink.attributes('target')).toBe('_blank')
  })

  it('Crates.io link has correct href', () => {
    const wrapper = mount(Footer)
    const cratesLink = wrapper.findAll('.footer-link')[2]
    expect(cratesLink.attributes('href')).toBe('https://crates.io/crates/simple_ssh')
    expect(cratesLink.attributes('target')).toBe('_blank')
  })

  it('all links have noopener and noreferrer', () => {
    const wrapper = mount(Footer)
    const links = wrapper.findAll('.footer-link')

    links.forEach(link => {
      expect(link.attributes('rel')).toContain('noopener')
      expect(link.attributes('rel')).toContain('noreferrer')
    })
  })

  it('links use proper href values', () => {
    const wrapper = mount(Footer)
    const links = wrapper.findAll('.footer-link')

    const expectedLinks = [
      'https://github.com/juliankahlert/simple_ssh',
      'https://docs.rs/simple_ssh',
      'https://crates.io/crates/simple_ssh'
    ]

    links.forEach((link, index) => {
      expect(link.attributes('href')).toBe(expectedLinks[index])
    })
  })
})
