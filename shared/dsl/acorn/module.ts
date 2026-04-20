import type { LangiumCoreServices, LangiumSharedCoreServices } from 'langium';
import { IndentationAwareLexer, IndentationAwareTokenBuilder } from 'langium';
import { AcornGeneratedModule, AcornGeneratedSharedModule } from './generated/module.js';

export { AcornGeneratedModule as AcornSharedModule } from './generated/module.js';

const indentTokenBuilder = new IndentationAwareTokenBuilder({
  indentTokenName: 'INDENT',
  dedentTokenName: 'DEDENT',
  whitespaceTokenName: 'WS',
  ignoreIndentationDelimiters: [['(', ')']],
});

export const AcornModule = {
  ...AcornGeneratedModule,
  parser: {
    ...AcornGeneratedModule.parser,
    TokenBuilder: () => indentTokenBuilder,
    Lexer: (services: LangiumCoreServices) => new IndentationAwareLexer(services),
    ParserConfig: () => ({
      maxLookahead: 2,
    }),
  },
} satisfies Record<string, unknown>;

export { AcornGeneratedModule, AcornGeneratedSharedModule };
