grammar Acorn

// ─── Entry ────────────────────────────────────────────────────────────────────

entry Script:
    (statements+=Statement)*;

// ─── Statements ───────────────────────────────────────────────────────────────

Statement:
    FindStatement | ForStatement | SetStatement | IfStatement | UseStatement;

// ─── find (SQL-like query) ────────────────────────────────────────────────────
//
// find first 3 npcs from modules[npcs] where strength < 3.
// find exactly 1 relationship from relation::player where npc = "Daniel".

FindStatement:
    'find' limit=FindLimit name=ID
    NestedFrom NestedWhere? '.' DEDENT*;

NestedFrom:
    INDENT* 'from' source=Expression;

NestedWhere:
    INDENT* 'where' INDENT* filter=Expression;

FindLimit:
    {infer FindFirst} 'first' count=NUMBER |
    {infer FindExactly} 'exactly' count=NUMBER;

// ─── for each ─────────────────────────────────────────────────────────────────
//
// for each npc in npcs:
//   ...

ForStatement:
    'for' 'each' var=ID 'in' iterable=Expression ':'
        INDENT body+=Statement+ DEDENT;

// ─── set ──────────────────────────────────────────────────────────────────────

SetStatement:
    'set' target=SetTarget expression=Expression '.';

SetTarget:
    {infer SetTarget} head=ID chain+=MemberChain* op=AssignOp;

MemberChain:
    '->' field=ID |
    '[' index=Expression ']';

AssignOp returns string:
    '=' | '+=' | '-=' | '*=' | '/=';

// ─── use ──────────────────────────────────────────────────────────────────────

UseStatement:
    'use' expression=Expression ('if' condition=Expression)? '.';

// ─── if / unless ──────────────────────────────────────────────────────────────

IfStatement:
    ('if' | 'unless') condition=Expression ':'
        INDENT then+=Statement+ DEDENT
    ('else' ':'
        INDENT else+=Statement+ DEDENT)?
;

// ─── Expressions ──────────────────────────────────────────────────────────────

infix Expression on PostfixExpr infers Expression:
    left assoc 'or'
  > left assoc 'and'
  > left assoc '==' | '=' | '!=' | '<' | '>' | '<=' | '>='
  > left assoc '+' | '-'
  > left assoc '*' | '/' | '%'
;

// Postfix: property access (a.b.c) and indexing (a[i])
PostfixExpr infers Expression:
    AtomExpr (parts+=PostfixPart)*;

PostfixPart infers Expression:
    {infer PropAccess} '->' property=ID |
    {infer IndexAccess} '[' index=Expression ']' |
    {infer QualifiedAccess} '::' property=ID;

AtomExpr infers Expression:
    {infer NotExpr} 'not' expr=PostfixExpr |
    {infer BooleanLiteral} value='true' |
    {infer BooleanLiteral} value='false' |
    {infer NumberLiteral} value=NUMBER |
    {infer StringLiteral} value=STRING |
    {infer VariableRef} name=ID |
    {infer ParenExpr} '(' inner=Expression ')';

// ─── Terminals ────────────────────────────────────────────────────────────────

// INDENT and DEDENT are synthetic tokens injected by IndentationAwareTokenBuilder.
// Their patterns here are only placeholders; the builder replaces them at runtime.
terminal INDENT: /indent/;
terminal DEDENT: /dedent/;

terminal ID: /[_a-zA-Z][\w]*/;
terminal NUMBER: /[0-9]+/;
terminal STRING: /"([^"\\]|\\.)*"|'([^'\\]|\\.)*'/;

hidden terminal WS: /[^\S\r\n]+/;
hidden terminal NEWLINE: /\r?\n/;
hidden terminal ML_COMMENT: /#[^\n\r]*/;
