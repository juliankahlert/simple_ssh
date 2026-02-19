import { describe, it, expect, vi, afterEach } from 'vitest'
import { mount, VueWrapper } from '@vue/test-utils'
import Header from './Header.vue'

let wrapper: VueWrapper<InstanceType<typeof Header>> | null = null;

afterEach(() => {
  if (wrapper) {
    wrapper.unmount();
  }
  document.body.style.overflow = '';
  vi.restoreAllMocks();
});

describe('Header', () => {
  it('renders logo with mark', () => {
    wrapper = mount(Header);
    const logo = wrapper.find('.logo');
    expect(logo.exists()).toBe(true);
    expect(logo.find('.logo-mark').text()).toBe('>');
  });

  it('renders logo text', () => {
    wrapper = mount(Header);
    expect(wrapper.find('.logo-text').text()).toBe('simple_ssh');
  });

  it('renders all navigation links', () => {
    wrapper = mount(Header);
    const links = wrapper.findAll('.nav-desktop .nav-link');
    expect(links.length).toBe(4);
    expect(links[0].text()).toBe('Features');
    expect(links[1].text()).toBe('Examples');
    expect(links[2].text()).toBe('Docs');
    expect(links[3].text()).toContain('GitHub');
  });

  it('GitHub link has correct href', () => {
    wrapper = mount(Header);
    const githubLink = wrapper.find('.nav-desktop .github-link');
    expect(githubLink.attributes('href')).toBe('https://github.com/juliankahlert/simple_ssh');
    expect(githubLink.attributes('target')).toBe('_blank');
  });

  it('shows mobile menu button on small screens', () => {
    wrapper = mount(Header);
    expect(wrapper.find('.mobile-menu-btn').exists()).toBe(true);
  });

  it('toggles mobile menu when button clicked', async () => {
    wrapper = mount(Header);
    const button = wrapper.find('.mobile-menu-btn');

    expect(wrapper.find('.nav-mobile').exists()).toBe(false);

    await button.trigger('click');
    expect(wrapper.find('.nav-mobile').exists()).toBe(true);

    await button.trigger('click');
    expect(wrapper.find('.nav-mobile').exists()).toBe(false);
  });

  it('locks body scroll when mobile menu opens', async () => {
    wrapper = mount(Header);
    const button = wrapper.find('.mobile-menu-btn');

    await button.trigger('click');
    expect(document.body.style.overflow).toBe('hidden');

    await button.trigger('click');
    expect(document.body.style.overflow).toBe('');
  });

  it('closes mobile menu when clicking outside', async () => {
    wrapper = mount(Header);
    const button = wrapper.find('.mobile-menu-btn');

    await button.trigger('click');
    expect(wrapper.find('.nav-mobile').exists()).toBe(true);

    const clickEvent = new MouseEvent('click', { bubbles: true });
    document.body.dispatchEvent(clickEvent);
    await wrapper.vm.$nextTick();

    expect(wrapper.find('.nav-mobile').exists()).toBe(false);
  });

  it('keeps mobile menu open when clicking inside header', async () => {
    wrapper = mount(Header);
    const button = wrapper.find('.mobile-menu-btn');

    await button.trigger('click');
    expect(wrapper.find('.nav-mobile').exists()).toBe(true);

    const header = wrapper.find('.header');
    const clickEvent = new MouseEvent('click', { bubbles: true });
    header.element.dispatchEvent(clickEvent);
    await wrapper.vm.$nextTick();

    expect(wrapper.find('.nav-mobile').exists()).toBe(true);
  });

  it('mobile navigation links close the menu when clicked', async () => {
    wrapper = mount(Header);
    const button = wrapper.find('.mobile-menu-btn');

    await button.trigger('click');
    expect(wrapper.find('.nav-mobile').exists()).toBe(true);

    const mobileLinks = wrapper.findAll('.nav-mobile .nav-link');
    await mobileLinks[0].trigger('click');
    expect(wrapper.find('.nav-mobile').exists()).toBe(false);
  });

  it('renders mobile navigation with all links', async () => {
    wrapper = mount(Header);
    const button = wrapper.find('.mobile-menu-btn');

    await button.trigger('click');
    const mobileLinks = wrapper.findAll('.nav-mobile .nav-link');
    expect(mobileLinks.length).toBe(4);
  });

  it('has correct aria-label on mobile menu button', async () => {
    wrapper = mount(Header);
    const button = wrapper.find('.mobile-menu-btn');
    expect(button.attributes('aria-label')).toBe('Open menu');

    await button.trigger('click');
    expect(button.attributes('aria-label')).toBe('Close menu');
  });
});
