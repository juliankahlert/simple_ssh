export interface Token {
  type: string;
  content: string;
}

// Keywords, types, and other token categories
const KEYWORDS = ['use', 'async', 'fn', 'let', 'mut', 'return', 'if', 'else', 'match', 'struct', 'enum', 'impl', 'pub', 'crate', 'mod', 'self', 'await'];
const TYPES = ['Session', 'Result', 'Option', 'String', 'Vec', 'HashMap', 'Box', 'Arc', 'Mutex', 'RwLock', 'Ok', 'Cli', 'Error'];

export function classifyWord(word: string): string {
  if (KEYWORDS.includes(word)) return 'keyword';
  if (TYPES.includes(word)) return 'type';
  return 'text';
}

export function tokenizeRust(line: string): Token[] {
  const tokens: Token[] = [];
  let remaining = line;

  while (remaining.length > 0) {
    const char = remaining.charAt(0);
    
    // Handle whitespace
    if (char.match(/\s/)) {
      const match = remaining.match(/^\s+/);
      if (match) {
        tokens.push({ type: 'space', content: match[0] });
        remaining = remaining.slice(match[0].length);
        continue;
      }
    }

    // Handle comments
    if (remaining.startsWith('//')) {
      tokens.push({ type: 'comment', content: remaining });
      remaining = '';
      continue;
    }

    // Handle string literals
    if (char === '"') {
      let strEnd = 1;
      let escaped = false;
      while (strEnd < remaining.length) {
        if (escaped) {
          escaped = false;
        } else if (remaining.charAt(strEnd) === '\\') {
          escaped = true;
        } else if (remaining.charAt(strEnd) === '"') {
          strEnd++;
          break;
        }
        strEnd++;
      }
      tokens.push({ type: 'string', content: remaining.slice(0, strEnd) });
      remaining = remaining.slice(strEnd);
      continue;
    }

    // Handle :: separator
    if (remaining.startsWith('::')) {
      tokens.push({ type: 'punct', content: '::' });
      remaining = remaining.slice(2);
      continue;
    }

    // Handle -> and =>
    if (remaining.startsWith('->')) {
      tokens.push({ type: 'punct', content: '->' });
      remaining = remaining.slice(2);
      continue;
    }
    if (remaining.startsWith('=>')) {
      tokens.push({ type: 'punct', content: '=>' });
      remaining = remaining.slice(2);
      continue;
    }

    // Handle multi-char comparison operators
    if (remaining.startsWith('>=') || remaining.startsWith('<=') || remaining.startsWith('==') || remaining.startsWith('!=')) {
      tokens.push({ type: 'punct', content: remaining.slice(0, 2) });
      remaining = remaining.slice(2);
      continue;
    }

    // Handle punctuation (single char)
    if (char.match(/[{}()\[\];:,.?!&|#@\+\-\*\/%<>=]/)) {
      tokens.push({ type: 'punct', content: char });
      remaining = remaining.slice(1);
      continue;
    }

    // Handle numbers (integers, floats, hex, binary, octal)
    if (char.match(/\d/) || (char === '.' && remaining.charAt(1)?.match(/\d/))) {
      const match = remaining.match(/^(0x[0-9a-fA-F_]+|0b[01_]+|0o[0-7_]+|\d[\d_]*(\.\d[\d_]*)?([eE][+-]?\d[\d_]*)?(f32|f64)?|\.\d[\d_]*([eE][+-]?\d[\d_]*)?(f32|f64)?)/);
      if (match) {
        tokens.push({ type: 'const', content: match[0] });
        remaining = remaining.slice(match[0].length);
        continue;
      }
    }

    // Handle identifiers and keywords
    if (char.match(/[a-zA-Z_]/)) {
      const match = remaining.match(/^[a-zA-Z_]\w*/);
      if (match) {
        const word = match[0];
        // Check if next char is ( to mark as function
        const nextChar = remaining.charAt(word.length);
        const afterNext = remaining.slice(word.length + 1);
        
        // Special case: word!(...) is a macro (function-like)
        if (nextChar === '!' && afterNext.startsWith('(')) {
          tokens.push({ type: 'macro', content: word + '!' });
          remaining = remaining.slice(word.length + 1);
          continue;
        } else if (nextChar === '(') {
          // Ok is a type even when followed by (
          if (word === 'Ok') {
            tokens.push({ type: 'type', content: word });
          } else {
            tokens.push({ type: 'func', content: word });
          }
        } else if (remaining.slice(word.length).startsWith('::')) {
          tokens.push({ type: 'path', content: word });
        } else {
          tokens.push({ type: classifyWord(word), content: word });
        }
        remaining = remaining.slice(word.length);
        continue;
      }
    }

    // Handle any other single character
    tokens.push({ type: 'text', content: char });
    remaining = remaining.slice(1);
  }

  return tokens;
}

export function useSyntaxHighlight() {
  function highlightRust(code: string): string {
    if (!code) return '';
    
    const lines = code.split('\n');
    const highlightedLines = lines.map(line => {
      const tokens = tokenizeRust(line);
      return tokens.map(token => `<span class="syn-${token.type}">${escapeHtml(token.content)}</span>`).join('');
    });
    
    return highlightedLines.join('\n');
  }

  return { highlightRust, tokenize: tokenizeRust };
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}
