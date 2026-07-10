import { describe, it, expect } from 'vitest';
import { tokenizeRust, type Token } from './useSyntaxHighlight';

describe('tokenizeRust', () => {

  it('correctly highlights use statements', () => {
    const line = 'use anyhow::Result;';
    const tokens = tokenizeRust(line);
    
    expect(tokens).toContainEqual({ type: 'keyword', content: 'use' });
    expect(tokens).toContainEqual({ type: 'path', content: 'anyhow' });
    expect(tokens).toContainEqual({ type: 'type', content: 'Result' });
    expect(tokens).toContainEqual({ type: 'punct', content: ';' });
  });

  it('correctly highlights simple_ssh import', () => {
    const line = 'use simple_ssh::Session;';
    const tokens = tokenizeRust(line);
    
    expect(tokens).toContainEqual({ type: 'keyword', content: 'use' });
    expect(tokens).toContainEqual({ type: 'path', content: 'simple_ssh' });
    expect(tokens).toContainEqual({ type: 'type', content: 'Session' });
  });

  it('correctly highlights mod declaration', () => {
    const line = 'mod cli;';
    const tokens = tokenizeRust(line);
    
    expect(tokens).toContainEqual({ type: 'keyword', content: 'mod' });
    expect(tokens).toContainEqual({ type: 'text', content: 'cli' });
    expect(tokens).toContainEqual({ type: 'punct', content: ';' });
  });

  it('correctly highlights cli import', () => {
    const line = 'use cli::Cli;';
    const tokens = tokenizeRust(line);
    
    expect(tokens).toContainEqual({ type: 'keyword', content: 'use' });
    expect(tokens).toContainEqual({ type: 'path', content: 'cli' });
    expect(tokens).toContainEqual({ type: 'type', content: 'Cli' });
  });

  it('correctly highlights attribute macro', () => {
    const line = '#[tokio::main]';
    const tokens = tokenizeRust(line);
    
    expect(tokens).toContainEqual({ type: 'punct', content: '#' });
    expect(tokens).toContainEqual({ type: 'punct', content: '[' });
    expect(tokens).toContainEqual({ type: 'path', content: 'tokio' });
    expect(tokens).toContainEqual({ type: 'punct', content: ']' });
  });

  it('correctly highlights async fn main', () => {
    const line = 'async fn main() -> Result<()> {';
    const tokens = tokenizeRust(line);
    
    expect(tokens).toContainEqual({ type: 'keyword', content: 'async' });
    expect(tokens).toContainEqual({ type: 'keyword', content: 'fn' });
    expect(tokens).toContainEqual({ type: 'func', content: 'main' });
    expect(tokens).toContainEqual({ type: 'type', content: 'Result' });
  });

  it('correctly highlights method chains with strings and ? operators', () => {
    const line = '    let args = Cli::with("host")?.and("user")?.and("passwd")?;';
    const tokens = tokenizeRust(line);
    
    // Should have two strings
    const strings = tokens.filter(t => t.type === 'string');
    expect(strings).toHaveLength(3);
    expect(strings[0]).toEqual({ type: 'string', content: '"host"' });
    expect(strings[1]).toEqual({ type: 'string', content: '"user"' });
    expect(strings[2]).toEqual({ type: 'string', content: '"passwd"' });
    
    // Should have ? operators as punctuation
    const questionMarks = tokens.filter(t => t.type === 'punct' && t.content === '?');
    expect(questionMarks).toHaveLength(3);
    
    // Should highlight method calls
    expect(tokens).toContainEqual({ type: 'func', content: 'with' });
    expect(tokens).toContainEqual({ type: 'func', content: 'and' });
  });

  it('correctly highlights unwrap() calls - NOT highlighting args as function', () => {
    const line = '        .with_host(&args.host.unwrap())';
    const tokens = tokenizeRust(line);
    
    // Should highlight with_host as function
    expect(tokens).toContainEqual({ type: 'func', content: 'with_host' });
    
    // Should highlight unwrap as function
    expect(tokens).toContainEqual({ type: 'func', content: 'unwrap' });
    
    // Should NOT highlight "args" as a function (it should be text)
    const argsToken = tokens.find(t => t.content === 'args');
    expect(argsToken?.type).not.toBe('func');
    expect(argsToken?.type).toBe('text');
  });

  it('correctly highlights unwrap_or() with number argument', () => {
    const line = '        .with_port(args.port.unwrap_or(22))';
    const tokens = tokenizeRust(line);
    
    expect(tokens).toContainEqual({ type: 'func', content: 'with_port' });
    expect(tokens).toContainEqual({ type: 'func', content: 'unwrap_or' });
    expect(tokens).toContainEqual({ type: 'const', content: '22' });
  });

  it('correctly highlights println! macro with string and formatting', () => {
    const line = '    println!("Exit code: {}", code);';
    const tokens = tokenizeRust(line);
    
    expect(tokens).toContainEqual({ type: 'macro', content: 'println!' });
    
    const strings = tokens.filter(t => t.type === 'string');
    expect(strings.length).toBeGreaterThan(0);
    
    // String should contain "Exit code: {}"
    const stringToken = strings[0];
    expect(stringToken.content).toContain('Exit code');
  });

  it('correctly highlights await? chains', () => {
    const line = '        .await?;';
    const tokens = tokenizeRust(line);
    
    expect(tokens).toContainEqual({ type: 'keyword', content: 'await' });
    expect(tokens).toContainEqual({ type: 'punct', content: '?' });
    expect(tokens).toContainEqual({ type: 'punct', content: ';' });
  });

  it('correctly highlights Ok(())', () => {
    const line = '    Ok(())';
    const tokens = tokenizeRust(line);
    
    expect(tokens).toContainEqual({ type: 'type', content: 'Ok' });
  });
});
