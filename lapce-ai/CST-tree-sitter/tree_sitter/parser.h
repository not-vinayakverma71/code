// Stub header for tree-sitter parser compilation
#ifndef TREE_SITTER_PARSER_H_
#define TREE_SITTER_PARSER_H_

#ifdef __cplusplus
extern "C" {
#endif

#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define ts_builtin_sym_error ((TSSymbol)-1)
#define ts_builtin_sym_end 0
#define TREE_SITTER_SERIALIZATION_BUFFER_SIZE 1024

typedef uint16_t TSStateId;
typedef uint16_t TSSymbol;
typedef uint16_t TSFieldId;
typedef struct TSLanguage TSLanguage;
typedef struct TSParser TSParser;
typedef struct TSTree TSTree;
typedef struct TSQuery TSQuery;
typedef struct TSQueryCursor TSQueryCursor;

typedef enum {
  TSParseActionTypeShift,
  TSParseActionTypeReduce,
  TSParseActionTypeAccept,
  TSParseActionTypeRecover,
} TSParseActionType;

typedef struct {
  TSStateId state;
  bool extra;
  bool repetition;
} TSLexMode;

typedef struct {
  TSParseActionType type:4;
  bool extra:1;
  bool repetition:1;
  bool shift_extra:1;
  uint8_t context:3;
  uint16_t to_state;
} TSParseAction;

typedef struct {
  uint16_t lex_state;
  uint16_t external_lex_state;
} TSParseActionEntry;

typedef struct {
  bool visible:1;
  bool named:1;
} TSSymbolMetadata;

typedef struct TSLexer TSLexer;

struct TSLexer {
  int32_t lookahead;
  TSSymbol result_symbol;
  void (*advance)(TSLexer *, bool);
  void (*mark_end)(TSLexer *);
  uint32_t (*get_column)(TSLexer *);
  bool (*is_at_included_range_start)(const TSLexer *);
  bool (*eof)(const TSLexer *);
};

typedef enum {
  TSLogTypeParse,
  TSLogTypeLex,
} TSLogType;

typedef struct {
  void *payload;
  const char *(*get)(void *, TSLogType);
} TSLogger;

typedef struct {
  uint32_t start;
  uint32_t end;
} TSRange;

typedef struct {
  TSRange range;
  uint32_t index;
} TSInput;

typedef struct {
  uint32_t row;
  uint32_t column;
} TSPoint;

typedef struct {
  uint32_t start_byte;
  uint32_t end_byte;
  TSPoint start_point;
  TSPoint end_point;
} TSInputEdit;

typedef struct TSNode {
  uint32_t context[4];
  const void *id;
  const TSTree *tree;
} TSNode;

typedef struct {
  const void *tree;
  const void *id;
  uint32_t context[2];
} TSTreeCursor;

typedef struct {
  TSNode node;
  uint32_t index;
} TSQueryCapture;

typedef struct {
  uint32_t id;
  uint16_t pattern_index;
  uint16_t capture_count;
  const TSQueryCapture *captures;
} TSQueryMatch;

typedef enum {
  TSQueryPredicateStepTypeDown,
  TSQueryPredicateStepTypeCapture,
  TSQueryPredicateStepTypeString,
  TSQueryPredicateStepTypeDone,
} TSQueryPredicateStepType;

typedef struct {
  TSQueryPredicateStepType type;
  uint32_t value_id;
} TSQueryPredicateStep;

typedef enum {
  TSQueryErrorNone = 0,
  TSQueryErrorSyntax,
  TSQueryErrorNodeType,
  TSQueryErrorField,
  TSQueryErrorCapture,
} TSQueryError;

#ifdef __cplusplus
}
#endif

#endif  // TREE_SITTER_PARSER_H_
