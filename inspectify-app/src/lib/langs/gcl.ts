import monaco from '../monaco';

export const GCL_LANGUAGE_ID = 'gcl';

monaco.languages.register({
	id: GCL_LANGUAGE_ID,
	extensions: ['gcl'],
	aliases: [],
	mimetypes: ['application/gcl']
});
monaco.languages.setLanguageConfiguration(GCL_LANGUAGE_ID, {
	comments: {
		lineComment: '//',
		blockComment: ['/*', '*/']
	},
	brackets: [
		['(', ')'],
		['{', '}'],
		['[', ']']
	],
	autoClosingPairs: [
		{ open: '[', close: ']' },
		{ open: '{', close: '}' },
		{ open: '(', close: ')' },
		{ open: "'", close: "'", notIn: ['string', 'comment'] },
		{ open: '"', close: '"', notIn: ['string'] }
	],
	surroundingPairs: [
		{ open: '{', close: '}' },
		{ open: '[', close: ']' },
		{ open: '(', close: ')' },
		{ open: '"', close: '"' },
		{ open: "'", close: "'" }
	],
	folding: {
		markers: {
			start: new RegExp('^\\s*#pragma\\s+region\\b'),
			end: new RegExp('^\\s*#pragma\\s+endregion\\b')
		}
	},
	wordPattern: /[a-zA-Z_@$ΣΛλ][a-zA-Z0-9_]*/
});
monaco.languages.setMonarchTokensProvider(GCL_LANGUAGE_ID, {
	defaultToken: '',
	brackets: [
		{ token: 'delimiter.curly', open: '{', close: '}' },
		{ token: 'delimiter.parenthesis', open: '(', close: ')' },
		{ token: 'delimiter.square', open: '[', close: ']' },
		{ token: 'delimiter.angle', open: '<', close: '>' }
	],

	keywords: ['if', 'fi', 'do', 'od'],
	operators: [
		'-',
		',',
		'->',
		':=',
		'!',
		'!=',
		'(',
		')',
		'{',
		'}',
		'*',
		'/',
		'^',
		'&&',
		'&',
		'+',
		'<',
		'<=',
		'=',
		'>',
		'>=',
		'||',
		'|'
	],
	tokenizer: {
		root: [
			[
				/[a-zA-Z_@$ΣΛλ][a-zA-Z0-9_]*/,
				{
					cases: {
						'@keywords': 'keyword',
						'@operators': 'operator',
						'@default': 'identifier'
					}
				}
			],
			{ include: '@whitespace' },
			[/[-,:=!*\/&+<>|]/, 'keyword.operator'],
			[/(\/\/).*$/, 'comment'],
			[/[{}()\[\]]/, '@brackets'],
			[/[0-9]+/, 'number']
		],
		whitespace: [
			[/[ \t\r\n]+/, ''],
			[/\/\*/, 'comment', '@comment'],
			[/\/\/.*\\$/, 'comment', '@linecomment'],
			[/\/\/.*$/, 'comment']
		],
		comment: [
			[/[^\/*]+/, 'comment'],
			[/\*\//, 'comment', '@pop'],
			[/[\/*]/, 'comment']
		],
		linecomment: [
			[/.*[^\\]$/, 'comment', '@pop'],
			[/[^]+/, 'comment']
		]
	}
});
