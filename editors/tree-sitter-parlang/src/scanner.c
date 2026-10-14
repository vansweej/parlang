#include "tree_sitter/parser.h"
#include <stdbool.h>
#include <stddef.h>

enum TokenType { BLOCK_COMMENT };

void *tree_sitter_parlang_external_scanner_create(void) { return NULL; }

void tree_sitter_parlang_external_scanner_destroy(void *payload) {
  (void)payload;
}

unsigned tree_sitter_parlang_external_scanner_serialize(void *payload,
                                                         char *buffer) {
  (void)payload;
  (void)buffer;
  return 0;
}

void tree_sitter_parlang_external_scanner_deserialize(void *payload,
                                                       const char *buffer,
                                                       unsigned length) {
  (void)payload;
  (void)buffer;
  (void)length;
}

bool tree_sitter_parlang_external_scanner_scan(void *payload, TSLexer *lexer,
                                                const bool *valid_symbols) {
  (void)payload;

  if (!valid_symbols[BLOCK_COMMENT]) {
    return false;
  }

  while (lexer->lookahead == ' ' || lexer->lookahead == '\t' ||
         lexer->lookahead == '\n' || lexer->lookahead == '\r') {
    lexer->advance(lexer, true);
  }

  if (lexer->lookahead != '{') {
    return false;
  }
  lexer->advance(lexer, false);

  if (lexer->lookahead != '-') {
    return false;
  }
  lexer->advance(lexer, false);

  unsigned depth = 1;

  for (;;) {
    if (lexer->eof(lexer)) {
      return false;
    }

    if (lexer->lookahead == '{') {
      lexer->advance(lexer, false);
      if (lexer->lookahead == '-') {
        lexer->advance(lexer, false);
        depth++;
        continue;
      }
      continue;
    }

    if (lexer->lookahead == '-') {
      lexer->advance(lexer, false);
      if (lexer->lookahead == '}') {
        lexer->advance(lexer, false);
        depth--;
        if (depth == 0) {
          lexer->mark_end(lexer);
          lexer->result_symbol = BLOCK_COMMENT;
          return true;
        }
        continue;
      }
      continue;
    }

    lexer->advance(lexer, false);
  }
}
