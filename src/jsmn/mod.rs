
#[derive(PartialEq)]
pub enum JsmnType {
    JsmnUndefined   = 0,
    JsmnObject      = 1 << 0,
    JsmnArray       = 1 << 1,
    JsmnString      = 1 << 2,
    JsmnPrimitive   = 1 << 3
}

pub enum JsmnError {
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

pub struct JsmnTok {
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
pub struct JsmnParser {
    pos: usize,       /* offset in the JSON string */
    tok_next: usize,   /* next token to allocate */
    tok_super: isize,  /* superior token node, e.g. parent object or array */
} 

/**
 * Allocates a fresh unused token from the token pool.
 */
fn jsmn_alloc_token<'life_of_parser, 'life_of_tokens>(
    parser: &'life_of_parser mut JsmnParser, 
    tokens: &'life_of_tokens mut Vec<JsmnTok>,
    num_tokens: usize) -> Option<&'life_of_tokens mut JsmnTok> {
    
    if parser.tok_next >= num_tokens {
        return None;
    } 
    
    let curr_tok = parser.tok_next;
    parser.tok_next += 1;
    let tok: &mut JsmnTok = &mut tokens[curr_tok] as &'life_of_tokens mut JsmnTok;
    tok.start = -1;
    tok.end = -1;
    tok.size = 0;
    tok.parent = -1;
    
    // ref coercion occurs here
    return Some(tok);
}

/**
 * Fills token type and boundaries.
 */
fn jsmn_fill_token(maybe_token: Option<&mut JsmnTok>,  jsmn_type: JsmnType, start: isize, end: isize) {
    if let Some(token) = maybe_token {
        token.jsmn_type = jsmn_type;
        token.start = start;
        token.end = end;
        token.size = 0;
    }
}

/**
 * Fills next available token with JSON primitive.
 */
pub fn jsmn_parse_primitive<'life_of_parser, 'life_of_tokens>(
    parser: &'life_of_parser mut JsmnParser, 
    js: &'_ [u8],
    len: usize, 
    tokens: &'life_of_tokens mut Vec<JsmnTok>, num_tokens: usize) -> Result<(), JsmnError> {
  
    let start = parser.pos;

    // let mut found = false;
    while parser.pos < len && js.get(parser.pos as usize) != Some(&b'\0') {
        match js[parser.pos] {
            // strict mode: b':'
            b'\t' | b'\r' | b'\n' | b' ' | b',' | b']' | b'}' =>  { 
                break;
            },
            _       =>  {}
        }

        if js.get(parser.pos) < Some(&32) || js.get(parser.pos) >= Some(&127) {
            parser.pos = start;
            return Err(JsmnError::JsmnErrorInval);
        }

        parser.pos += 1;
    }
    
    /*
     * we'll deal with this later
    if !found && cfg!(JSMN_STRICT) {
        parser.pos = start;
        return Err(JsmnError::JsmnErrorPart); 
    }
    */

    if tokens.len() == 0 {
        parser.pos -= 1;
        return Ok(());
    }
    
    let token = jsmn_alloc_token(parser, tokens, num_tokens);

    if token.is_none() {
        parser.pos = start;
        return Err(JsmnError::JsmnErrorNoMem);
    }

    jsmn_fill_token(token.into(), JsmnType::JsmnPrimitive, start as isize, parser.pos as isize);
    
    /* In strict mode primitive must be followed by a comma/object/array */
    /*
    if cfg!(JSMN_PARENT_LINKS) {
        token.parent = parser.tok_super;
    }
    */
    
    parser.pos -= 1;
  
    return Ok(());
}


/**
 * Fills next token with JSON string.
 */
pub fn jsmn_parse_string(parser: &mut JsmnParser, js: &[u8],
                        len: usize, tokens: &mut Vec<JsmnTok>,
                        num_tokens: usize) -> Result<(), JsmnError> {
    
    let start = parser.pos;
  
    /* Skip starting quote */
    parser.pos += 1;

    while parser.pos < len && js.get(parser.pos) != Some(&b'\0') {
        let c = *js.get(parser.pos).unwrap();
        
        if c == b'\"' {
            if tokens.len() == 0 {
                return Ok(());
            }

            let mut token = jsmn_alloc_token(parser, tokens, num_tokens);
        
            if token.is_none() {
                parser.pos = start;
                return Err(JsmnError::JsmnErrorNoMem);
            }

            jsmn_fill_token(token.as_deref_mut(), JsmnType::JsmnString, (start + 1) as isize,parser.pos as isize);

            if token.is_none() {
                parser.pos = start;
                return Err(JsmnError::JsmnErrorNoMem);
            }

           jsmn_fill_token(token.into(), JsmnType::JsmnString, (start + 1) as isize, parser.pos as isize);

           /*
            * compile time conditional
            * token.parent = parser.tok_super;
            */

           return Ok(());
        }
   
        /* Backslash: Quoted symbol expected */
        if c == b'\\' && (parser.pos + 1 < len) {
          // int i;
          parser.pos += 1;

          match js.get(parser.pos).unwrap() {
            /* Allowed escaped symbols */
            b'\"' |  b'/' | b'\\' | b'b' | b'f' | b'r' | b'n' | b't'    => { },
            b'u'                                                        => {
                parser.pos += 1;
                for _ in 0..4 {
                    if parser.pos >= len || *js.get(parser.pos).unwrap() == b'\0' {
                        break; 
                    }
                    /* If it isn't a hex character we have an error */
                    if !((*js.get(parser.pos).unwrap() >= 48 && *js.get(parser.pos).unwrap() <= 57) ||   /* 0-9 */
                        (*js.get(parser.pos).unwrap() >= 65 && *js.get(parser.pos).unwrap() <= 70) ||   /* A-F */
                        (*js.get(parser.pos).unwrap() >= 97 && *js.get(parser.pos).unwrap() <= 102)) { /* a-f */
                        parser.pos = start;
                        return Err(JsmnError::JsmnErrorInval);
                    }
                    parser.pos += 1;
                } 
                parser.pos -= 1;
            }
            /* Unexpected symbol */
            _                                                           => {
                parser.pos = start;
                return Err(JsmnError::JsmnErrorPart)
            } 
          }
        }
    }
    
    parser.pos = start;
    
    return Err(JsmnError::JsmnErrorPart);
}

pub fn jsmn_parse(parser: &mut JsmnParser, js: &[u8],
    len: usize, tokens: &mut Vec<JsmnTok>, num_tokens: usize) -> Result<(), JsmnError> {
    let mut count = parser.tok_next;
    let mut i = 0 as isize;

    while parser.pos < len && js.get(parser.pos).unwrap() != &b'\0' {
        
        let c= js[parser.pos]; 
        match c {
            b'{' | b'[' => 'this_branch: {
                count += 1;

                if tokens.len() == 0 {
                    // exit out of this match branch
                    break 'this_branch;
                }

                if let Some(token) = jsmn_alloc_token(parser, tokens, num_tokens) {
                    token.jsmn_type = match c {
                        b'{' => JsmnType::JsmnObject,
                        b'[' => JsmnType::JsmnArray,
                        _    => JsmnType::JsmnUndefined
                    };

                    token.start = parser.pos as isize;
                    token.size = parser.tok_next + 1;

                    if parser.tok_super != -1 {
                        let t = &mut tokens[parser.tok_super as usize];
                    
                        /*
                         * 
                         * #ifdef JSMN_STRICT
                         * if (t.type == JsmnType::JsmnObject) {
                         *  return Err(JsmnError::JsmnErrorInval);
                         * }
                         * #endif
                         * */
                        t.size += 1;
                        
                        /*
                         * #ifdef JSMN_PARENT_LINKS
                         * token.parent = parent.tok_super;
                         * #endif
                         * */
                    }
                    
                    parser.tok_super = parser.tok_next  as isize - 1;
                } else {
                    return Err(JsmnError::JsmnErrorNoMem);
                }
            },
            b'}' | b']' => 'this_branch: {
                if tokens.len() == 0 {
                    break 'this_branch;
                }

                // #ifdef PARENT_LINKS
                /* 
                if parser.tok_next < 1 {
                    return Err(JsmnError::JsmnErrorInval);
                }
            
                let mut token = &mut tokens[parser.tok_next - 1];
                let c_type=  match c {
                    b'}' => JsmnType::JsmnObject,
                    b']' => JsmnType::JsmnArray,
                    _    => JsmnType::JsmnUndefined
                };

                loop {
                    if token.start != -1 && token.end != -1 {
                        if token.jsmn_type != c_type {
                            return Err(JsmnError::JsmnErrorInval); 
                        } 
                        token.end = parser.pos as isize + 1;
                        parser.tok_super = token.parent;
                        break;
                    }

                    if token.parent != -1 {
                        if token.jsmn_type != c_type || parser.tok_super == -1 {
                            return Err(JsmnError::JsmnErrorInval);
                        }
                        break;
                    }
                    
                    let idx = token.parent as usize;
                    token = &mut tokens[idx];
                }
                */
            
                // #else for previous ifdef 
                i = parser.tok_next as isize - 1;
                while i >= 0 {
                    let token = &mut tokens[i as usize];
                    unimplemented!(); 
                    i -= 1;
                }
                    
            },
            _           => {}
        }
        
        parser.pos += 1;
    }

    Ok(())
}
