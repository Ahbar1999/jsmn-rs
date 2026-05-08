enum JsmnType {
    JsmnUndefined   = 0,
    JsmnObject      = 1 << 0,
    JsmnArray       = 1 << 1,
    JsmnString      = 1 << 2,
    JsmnPrimitive   = 1 << 3
}

enum JsmnError {
    /* Not enough tokens were provided */
    JsmnErrorNoMem = -1,
    /* Invalid character inside JSON string */
    JsmnErrorInval = -2,
    /* The string is not a full JSON packet, more bytes expected */
    JsmnErrorPart = -3,
}

/**
 * JSON token description.
 * type		type (object, array, string etc.)
 * start	start position in JSON data string
 * end		end position in JSON data string
 */

struct JsmnTok {
  jsmn_type: JsmnType,
  start: isize,
  end: isize,
  size: usize,
  parent: isize 
}

/**
 * JSON parser. Contains an array of token blocks available. Also stores
 * the string being parsed now and current position in that string.
 */
struct JsmnParser {
  pos: usize,       /* offset in the JSON string */
  tok_next: usize,   /* next token to allocate */
  tok_super: isize,  /* superior token node, e.g. parent object or array */
} 

/**
 * Allocates a fresh unused token from the token pool.
 */
fn jsmn_alloc_token<'life_of_parser, 'life_of_token, 'life_of_func>(
    parser: &'life_of_parser mut JsmnParser, 
    tokens: &'life_of_func mut Vec<&'life_of_token mut JsmnTok>,
    num_tokens: usize) -> Option<&'life_of_token JsmnTok> 
    where 'life_of_func: 'life_of_token {
    
    if parser.tok_next >= num_tokens {
        return None;
    } 
    
    let curr_tok = parser.tok_next;
    parser.tok_next += 1;
    let tok: &mut JsmnTok = &mut tokens[curr_tok] as &'life_of_token mut JsmnTok;
    tok.start = -1;
    tok.end = -1;
    tok.size = 0;
    tok.parent = -1;
      
    return Some(tok);
}
