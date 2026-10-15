module.exports = grammar({
  name: 'parlang',

  word: $ => $.identifier,

  extras: $ => [/\s/, $.doc_comment, $.line_comment, $.block_comment],

  externals: $ => [$.block_comment],

  rules: {
    source_file: $ => repeat($._item),

    _item: $ => choice(
      $.let_binding,
      $.fun_binding,
      $.rec_binding,
      $.data_head,
      $.type_head,
      $.field_label,
      $.projection,
      $.boolean,
      $.float,
      $.integer,
      $.char,
      $.string,
      $.constructor,
      $.identifier,
      $.wildcard,
      $.operator,
      'in',
      'if',
      'then',
      'else',
      'match',
      'with',
      'load',
      '(',
      ')',
      '{',
      '}',
      ',',
      ';',
      '|',
      ':',
      '=',
      '->',
    ),

    let_binding: $ => seq('let', field('name', $.identifier)),

    fun_binding: $ => seq('fun', optional('('), field('parameter', $.identifier)),

    rec_binding: $ => seq('rec', field('name', $.identifier)),

    data_head: $ => seq(
      'data',
      field('name', $.constructor),
      repeat(field('parameter', $.identifier)),
      '=',
    ),

    type_head: $ => seq('type', field('name', $.constructor)),

    field_label: $ => prec(1, seq(field('name', $.identifier), ':')),

    projection: $ => seq('.', field('field', choice($.integer, $.identifier))),

    doc_comment: $ => token(prec(2, /--[ \t]*[|^].*/)),

    line_comment: $ => token(prec(1, /--.*/)),

    float: $ => /-?\d+\.\d+/,

    integer: $ => /-?\d+/,

    identifier: $ => /[a-z][a-zA-Z0-9_]*/,

    constructor: $ => /[A-Z][a-zA-Z0-9_]*/,

    wildcard: $ => '_',

    boolean: $ => choice('true', 'false'),

    operator: $ => choice('+', '-', '*', '/', '==', '!=', '<=', '>=', '<', '>'),

    escape_sequence: $ => token.immediate(/\\[ntr\\"'0]/),

    char: $ => seq(
      "'",
      choice(token.immediate(/[^'\\]/), $.escape_sequence),
      token.immediate("'"),
    ),

    string: $ => seq(
      '"',
      repeat(choice(token.immediate(prec(1, /[^"\\]+/)), $.escape_sequence)),
      token.immediate('"'),
    ),
  },
});
