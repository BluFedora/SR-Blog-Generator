//
// Author: Shareef Abdoul-Raheem
// File:   ast.rs
//

use crate::Lexer;
use crate::Token;
use crate::Token::BoolLiteral;
use crate::Token::Character;
use crate::Token::EndOfFile;
use crate::Token::Error;
use crate::Token::NumberLiteral;
use crate::Token::StringLiteral;
use crate::Token::Tag;
use crate::Token::Text;
use crate::TokenTag;
use crate::TokenText;

use std::collections::HashMap;

// Ast Nodes

pub struct AstNodeRoot {
  pub children: Vec<Box<dyn IAstNode>>,
}

pub struct AstNodeTag {
  pub text: String,
  pub children: Vec<Box<dyn IAstNode>>,
  pub attributes: HashMap<String, AstLiteral>,
}

pub struct AstNodeText {
  pub text: String,
}

#[derive(Debug)]
pub enum AstLiteral {
  Str(String),
  Float(f64),
  Bool(bool),
}

pub trait IAstNode {
  fn visit(&self, parser: &mut Parser);
}

pub enum ASTNode {
  Root(AstNodeRoot),
  Tag(AstNodeTag),
  Text(AstNodeText),
  Literal(AstLiteral),
}

fn make_empty_token_text() -> Token {
  return Token::Text(TokenText {
    line_no_start: 0,
    line_no_end_with_content: 0,
    line_no_end: 0,
    text: "".to_string(),
  });
}


// Parser

pub struct Parser {
  lexer: Lexer,
  current_token: Token,
  pub error_log: Vec<String>,
}

impl Parser {
  pub fn new(lex: Lexer) -> Self {
    Parser {
      lexer: lex,
      current_token: Token::EndOfFile(),
      error_log: Vec::new(),
    }
  }

  pub fn parse(&mut self) -> Option<Box<dyn IAstNode>> {
    let mut root_node = AstNodeRoot {
      children: Vec::new(),
    };

    self.advance_token();
    self.parse_impl(&mut root_node.children);

    return if self.error_log.is_empty() {
      Some(Box::new(ASTNode::Root(root_node)))
    } else {
      None
    };
  }

  pub fn parse_impl(&mut self, parent_child_list: &mut Vec<Box<dyn IAstNode>>) {
    loop {
      let current_token = self.current_token.clone();

      match current_token {
        Tag(ref tt) => {
          let tt_node = self.parse_tag_block(&tt);

          if tt_node.is_ok() {
            parent_child_list.push(tt_node.unwrap());
          }
        }
        StringLiteral(ref str_lit) => {
          let child_node = Box::new(ASTNode::Literal(AstLiteral::Str(str_lit.clone())));
          self.advance_token();

          parent_child_list.push(child_node);
        }
        NumberLiteral(number) => {
          let child_node = Box::new(ASTNode::Literal(AstLiteral::Float(number)));
          self.advance_token();

          parent_child_list.push(child_node);
        }
        BoolLiteral(value) => {
          let child_node = Box::new(ASTNode::Literal(AstLiteral::Bool(value)));
          self.advance_token();

          parent_child_list.push(child_node);
        }
        Text(ref txt) => {
          let child_node = Box::new(ASTNode::Text(AstNodeText {
            text: txt.text.clone(),
          }));
          self.advance_token();

          parent_child_list.push(child_node);
        }
        Character(_value) => {
          break;
        }
        Error(err_msg) => {
          self.error_panic(format!("Tokenizer {}", err_msg));
        }
        EndOfFile() => {
          break;
        }
      }
    }
  }

  fn token_to_ast_literal(tok: Token) -> AstLiteral {
    match tok {
      StringLiteral(ref str_lit) => return AstLiteral::Str(str_lit.clone()),
      NumberLiteral(number) => return AstLiteral::Float(number),
      BoolLiteral(value) => return AstLiteral::Bool(value),
      _ => panic!("The token was not a literal"),
    }
  }

  fn parse_tag_block(&mut self, tag: &TokenTag) -> Result<Box<dyn IAstNode>, String> {
    let mut tag_node = Box::new(AstNodeTag::new(tag.text.clone()));

    self.advance_token();

    if self.expect(&Token::Character('(')) {
      while !self.expect(&Token::Character(')')) {
        let variable_name = self.current_token.clone();

        if !self.expect(&make_empty_token_text()) {
          self.error_panic(format!(
            "Variable must be a string name but got {}",
            variable_name
          ));
        }

        if !self.expect(&Token::Character('=')) {
          self.error_panic(format!("Expected assignment after '{}'.", variable_name));
        }

        let literal_value = self.current_token.clone();

        if literal_value.is_literal() {
          self.advance_token();

          let var_name_as_str = match variable_name {
            Token::Text(value) => value.text,
            _ => panic!("The variable must be a text node."),
          };

          tag_node
            .attributes
            .insert(var_name_as_str, Parser::token_to_ast_literal(literal_value));
        } else {
          self.error_panic(format!(
            "'{}' must be assigned a literal value.",
            variable_name
          ));
        }

        self.expect(&Token::Character(','));
      }
    }

    if self.require(&Token::Character('{')) {
      while !self.expect(&Token::Character('}')) {
        self.parse_impl(&mut tag_node.children);
      }
    }

    return Ok(tag_node);
  }

  fn current_token_is(&self, token: &Token) -> bool {
    let current_type = std::mem::discriminant(&self.current_token);
    let token_type = std::mem::discriminant(token);

    if current_type == token_type {
      if token_type != std::mem::discriminant(&Token::Character('_'))
        || self.current_token == *token
      {
        return true;
      }
    }

    return false;
  }

  fn require(&mut self, token: &Token) -> bool {
    if self.current_token_is(token) {
      self.advance_token();
      return true;
    }

    self.error_panic(format!(
      "Expected {} but got {}",
      *token, self.current_token
    ));
    // Prevent infinite loops by just returning true when at the end of a file.
    return self.current_token == Token::EndOfFile();
  }

  fn expect(&mut self, token: &Token) -> bool {
    if self.current_token_is(token) {
      self.advance_token();
      return true;
    }

    let is_at_end_of_file = self.current_token == Token::EndOfFile();

    // Prevent infinite loops by just returning true when at the end of a file.
    if is_at_end_of_file {
      self.error_panic(format!(
        "Unexpected eof of while searching for {}",
        *token
      ));
    }

    return is_at_end_of_file;
  }

  fn advance_token(&mut self) -> &Token {
    self.current_token = self.lexer.get_next_token();
    &self.current_token
  }

  fn error_panic(&mut self, message: String) {
    // Advance the token as not to get stuck in infinite loops.
    self.advance_token();
    self
      .error_log
      .push(format!("Line({}): {}", self.lexer.line_no, message));
  }
}

// Node Impl

impl AstNodeTag {
  pub fn new(text: String) -> Self {
    Self {
      text: text,
      children: Vec::new(),
      attributes: Default::default(),
    }
  }
}

impl IAstNode for AstNodeRoot {
  fn visit(&self, parser: &mut Parser) {
    for child in &self.children {
      child.visit(parser);
    }
  }
}

impl IAstNode for AstNodeTag {
  fn visit(&self, parser: &mut Parser) {
    println!("Tag: {} {{", self.text);

    if !self.attributes.is_empty() {
      println!("Attributes: ");

      for attrib in &self.attributes {
        println!("  {} = {:?}", attrib.0, attrib.1);
      }
    }

    for child in &self.children {
      child.visit(parser);
    }

    println!("}}");
  }
}

impl IAstNode for AstNodeText {
  fn visit(&self, _parser: &mut Parser) {
    println!("TEXT: {}", self.text);
  }
}

impl IAstNode for AstLiteral {
  fn visit(&self, _parser: &mut Parser) {
    println!("Literal: {:?}", self);
  }
}

impl IAstNode for ASTNode {
  fn visit(&self, parser: &mut Parser) {
    match self {
      ASTNode::Root(r) => r.visit(parser),
      ASTNode::Tag(t) => t.visit(parser),
      ASTNode::Text(t) => t.visit(parser),
      ASTNode::Literal(l) => l.visit(parser),
    }
  }
}
