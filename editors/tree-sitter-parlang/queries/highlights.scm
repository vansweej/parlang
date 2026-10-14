["let" "in" "data" "type"] @keyword
["if" "then" "else" "match" "with"] @keyword.conditional
["fun" "rec"] @keyword.function
"load" @keyword.import
(boolean) @boolean
(integer) @number
(float) @number.float
(char) @character
(string) @string
(escape_sequence) @string.escape
(line_comment) @comment
(block_comment) @comment
(doc_comment) @comment.documentation
(constructor) @constructor
(identifier) @variable
(wildcard) @variable.builtin
(operator) @operator
["(" ")" "{" "}"] @punctuation.bracket
["," ";" ":" "|" "->" "=" "."] @punctuation.delimiter

(let_binding name: (identifier) @variable)
(fun_binding parameter: (identifier) @variable.parameter)
(rec_binding name: (identifier) @function)
(data_head name: (constructor) @type.definition)
(data_head parameter: (identifier) @type)
(type_head name: (constructor) @type.definition)
(field_label name: (identifier) @property)
(projection field: (identifier) @property)
(projection field: (integer) @property)
