const subscriptMap: Record<string, string | void> = {
  '0': '₀',
  '1': '₁',
  '2': '₂',
  '3': '₃',
  '4': '₄',
  '5': '₅',
  '6': '₆',
  '7': '₇',
  '8': '₈',
  '9': '₉',
};
export const toSubscript = (str: string) =>
  str
    .split('')
    .map((char) => subscriptMap[char] || char)
    .join('');

type CharClass = 'Fst' | 'Alp' | 'Num' | 'Oth' | 'Lst';

const classifyChar = (c: string): CharClass =>
  [
    /[a-zA-Z]/.test(c) && 'Alp',
    /\d/.test(c) && 'Num',
    c === '▷' && 'Fst',
    c === '◀' && 'Lst',
    'Oth',
  ].find(Boolean) as CharClass;

const naturalSort = (a: string, b: string) => {
  const aC = Array.from(a).map(classifyChar);
  const bC = Array.from(b).map(classifyChar);

  for (let i = 0; i < Math.min(aC.length, bC.length); i++) {
    const [x, y] = [aC[i], bC[i]];
    if (x === y) continue;

    if (x === 'Fst') return -1;
    if (y === 'Fst') return 1;
    if (x === 'Lst') return 1;
    if (y === 'Lst') return -1;
    if (x === 'Alp' && y === 'Num') return -1;
    if (x === 'Num' && y === 'Alp') return 1;
    if (x === 'Alp' && y === 'Oth') return -1;
    if (x === 'Oth' && y === 'Alp') return 1;
    if (x === 'Num' && y === 'Oth') return -1;
    if (x === 'Oth' && y === 'Num') return 1;
  }

  return aC.length - bC.length;
};

export const sortNodes = <T>(nodes: [string, T][]): [string, T][] =>
  nodes.sort(([a], [b]) => naturalSort(a, b));
