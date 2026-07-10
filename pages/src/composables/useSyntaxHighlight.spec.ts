import { describe, it, expect } from 'vitest';
import { useSyntaxHighlight } from './useSyntaxHighlight';

describe('useSyntaxHighlight', () => {
  const { highlightRust } = useSyntaxHighlight();

  it('returns highlightRust function', () => {
    expect(typeof highlightRust).toBe('function');
  });

  it('wraps keywords in syn-keyword span', () => {
    const input = 'use std::io';
    const result = highlightRust(input);
    expect(result).toContain('<span class="syn-keyword">use</span>');
  });

  it('wraps multiple keywords in syn-keyword span', () => {
    const input = 'async fn main()';
    const result = highlightRust(input);
    expect(result).toContain('<span class="syn-keyword">async</span>');
    expect(result).toContain('<span class="syn-keyword">fn</span>');
  });

  it('wraps types in syn-type span', () => {
    const input = 'let result: Result<(), Error>';
    const result = highlightRust(input);
    expect(result).toContain('<span class="syn-type">Result</span>');
    expect(result).toContain('<span class="syn-type">Error</span>');
  });

  it('wraps strings in syn-string span', () => {
    const input = 'println!("Hello, world!")';
    const result = highlightRust(input);
    expect(result).toContain('<span class="syn-string">&quot;Hello, world!&quot;</span>');
  });

  it('wraps comments in syn-comment span', () => {
    const input = '// This is a comment';
    const result = highlightRust(input);
    expect(result).toContain('<span class="syn-comment">// This is a comment</span>');
  });

  it('wraps macros in syn-macro span', () => {
    const input = 'println!("test")';
    const result = highlightRust(input);
    expect(result).toContain('<span class="syn-macro">println!</span>');
  });

  it('wraps paths in syn-path span', () => {
    const input = 'use simple_ssh::Session';
    const result = highlightRust(input);
    expect(result).toContain('<span class="syn-path">simple_ssh</span>');
  });

  it('handles complex code with multiple token types', () => {
    const input = `use simple_ssh::Session;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let session = Session::builder()
        .connect("localhost", 22)
        .await?;
    Ok(())
}`;

    const result = highlightRust(input);
    expect(result).toContain('<span class="syn-keyword">use</span>');
    expect(result).toContain('<span class="syn-path">simple_ssh</span>');
    expect(result).toContain('<span class="syn-type">Session</span>');
    expect(result).toContain('<span class="syn-keyword">async</span>');
    expect(result).toContain('<span class="syn-keyword">fn</span>');
    expect(result).toContain('<span class="syn-path">anyhow</span>');
    expect(result).toContain('<span class="syn-type">Result</span>');
    expect(result).toContain('<span class="syn-keyword">let</span>');
    expect(result).toContain('<span class="syn-string">&quot;localhost&quot;</span>');
  });

  it('escapes HTML special characters', () => {
    const input = 'let x = 1 < 2;';
    const result = highlightRust(input);
    expect(result).toContain('&lt;');
    expect(result).not.toContain('<span class="syn-keyword">1 < 2</span>');
  });

  it('returns empty string for empty input', () => {
    const result = highlightRust('');
    expect(result).toBe('');
  });

  it('wraps constants in syn-const span', () => {
    const input = 'let x = 42;';
    const result = highlightRust(input);
    expect(result).toContain('<span class="syn-const">42</span>');
  });

  it('wraps punctuation in syn-punct span', () => {
    const input = 'fn main() { }';
    const result = highlightRust(input);
    expect(result).toContain('<span class="syn-punct">(</span>');
    expect(result).toContain('<span class="syn-punct">)</span>');
    expect(result).toContain('<span class="syn-punct">{</span>');
    expect(result).toContain('<span class="syn-punct">}</span>');
  });

  it('handles else keyword', () => {
    const input = 'if x > 0 { 1 } else { 2 }';
    const result = highlightRust(input);
    expect(result).toContain('<span class="syn-keyword">if</span>');
    expect(result).toContain('<span class="syn-keyword">else</span>');
  });

  it('handles return keyword', () => {
    const input = 'return Ok(())';
    const result = highlightRust(input);
    expect(result).toContain('<span class="syn-keyword">return</span>');
  });

  it('handles mut keyword', () => {
    const input = 'let mut x = 5';
    const result = highlightRust(input);
    expect(result).toContain('<span class="syn-keyword">let</span>');
    expect(result).toContain('<span class="syn-keyword">mut</span>');
  });

  it('handles anyhow path', () => {
    const input = 'use anyhow::Result';
    const result = highlightRust(input);
    expect(result).toContain('<span class="syn-path">anyhow</span>');
  });
});