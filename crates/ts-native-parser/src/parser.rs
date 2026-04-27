use std::mem;

use crate::ast::{
    AssignmentStatement, BinaryExpression, BinaryOperator, BindingKind, BlockStatement,
    BoolLiteral, CallExpression, ConsoleLogExpression, DoubleLiteral, Expression,
    ExpressionStatement,
    FunctionDeclaration, FunctionParameter, Identifier, IntegerLiteral, Program, ReturnStatement,
    Statement, TypeName, UnaryExpression, UnaryOperator, VariableDeclaration, WhileStatement,
};
use crate::{ParseError, Span, Token, TokenKind};

pub struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    current: usize,
}

impl<'a> Parser<'a> {
    #[must_use]
    pub fn new(source: &'a str, tokens: Vec<Token>) -> Self {
        Self {
            source,
            tokens,
            current: 0,
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let has_module_preamble = self.parse_module_preamble()?;
        let mut statements = Vec::new();

        while !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }

        let span = Span::new(0, self.peek().span.start);

        Ok(Program {
            has_module_preamble,
            statements,
            span,
        })
    }

    fn parse_module_preamble(&mut self) -> Result<bool, ParseError> {
        if !self.check_simple(&TokenKind::Export) {
            return Ok(false);
        }

        self.advance();
        self.expect_kind(&TokenKind::LeftBrace, "expected '{' after 'export'")?;
        self.expect_kind(&TokenKind::RightBrace, "expected '}' after 'export {'")?;
        self.expect_kind(&TokenKind::Semicolon, "expected ';' after 'export {}'")?;

        Ok(true)
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        if self.match_simple(&TokenKind::Const) {
            return self
                .parse_variable_declaration(BindingKind::Const)
                .map(Statement::VariableDeclaration);
        }

        if self.match_simple(&TokenKind::Let) {
            return self
                .parse_variable_declaration(BindingKind::Let)
                .map(Statement::VariableDeclaration);
        }

        if self.match_simple(&TokenKind::Function) {
            return self
                .parse_function_declaration()
                .map(Statement::FunctionDeclaration);
        }

        if self.match_simple(&TokenKind::Return) {
            return self.parse_return_statement().map(Statement::Return);
        }

        if self.match_simple(&TokenKind::While) {
            return self.parse_while_statement().map(Statement::While);
        }

        if self.match_simple(&TokenKind::LeftBrace) {
            let open_brace = self.previous();
            return self.parse_block_statement(open_brace).map(Statement::Block);
        }

        if self.is_assignment_start() {
            return self.parse_assignment_statement().map(Statement::Assignment);
        }

        if self.is_expression_statement_start() {
            return self.parse_expression_statement().map(Statement::Expression);
        }

        let token = self.peek();
        Err(ParseError::new(
            self.source,
            token.span,
            format!("expected statement, found {}", token.kind.describe()),
        ))
    }

    fn parse_function_declaration(&mut self) -> Result<FunctionDeclaration, ParseError> {
        let start = self.previous().span;
        let name = self.expect_identifier("expected function name after 'function'")?;

        self.expect_kind(&TokenKind::LeftParen, "expected '(' after function name")?;
        let parameters = self.parse_function_parameters()?;
        self.expect_kind(&TokenKind::RightParen, "expected ')' after parameter list")?;
        self.expect_kind(
            &TokenKind::Colon,
            "expected ':' before function return type",
        )?;
        let return_type = self.parse_type_name()?;
        let open_brace =
            self.expect_kind(&TokenKind::LeftBrace, "expected '{' before function body")?;
        let body = self.parse_block_statement(open_brace)?;

        Ok(FunctionDeclaration {
            name,
            parameters,
            return_type,
            span: start.through(body.span),
            body,
        })
    }

    fn parse_variable_declaration(
        &mut self,
        kind: BindingKind,
    ) -> Result<VariableDeclaration, ParseError> {
        let start = self.previous().span;
        let name = self.expect_identifier("expected binding name after declaration keyword")?;
        let type_annotation = if self.match_simple(&TokenKind::Colon) {
            Some(self.parse_type_name()?)
        } else {
            None
        };

        self.expect_kind(&TokenKind::Assign, "expected '=' in variable declaration")?;
        let initializer = self.parse_expression()?;
        let semicolon = self.expect_kind(
            &TokenKind::Semicolon,
            "expected ';' after variable declaration",
        )?;

        Ok(VariableDeclaration {
            kind,
            name,
            type_annotation,
            initializer,
            span: start.through(semicolon.span),
        })
    }

    fn parse_assignment_statement(&mut self) -> Result<AssignmentStatement, ParseError> {
        let target = self.expect_identifier("expected assignment target")?;
        self.expect_kind(&TokenKind::Assign, "expected '=' in assignment")?;
        let value = self.parse_expression()?;
        let semicolon = self.expect_kind(&TokenKind::Semicolon, "expected ';' after assignment")?;

        Ok(AssignmentStatement {
            span: target.span.through(semicolon.span),
            target,
            value,
        })
    }

    fn parse_expression_statement(&mut self) -> Result<ExpressionStatement, ParseError> {
        let expression = self.parse_expression()?;
        let semicolon = self.expect_kind(&TokenKind::Semicolon, "expected ';' after expression")?;

        Ok(ExpressionStatement {
            span: expression.span().through(semicolon.span),
            expression,
        })
    }

    fn parse_return_statement(&mut self) -> Result<ReturnStatement, ParseError> {
        let start = self.previous().span;

        if self.match_simple(&TokenKind::Semicolon) {
            return Ok(ReturnStatement {
                value: None,
                span: start.through(self.previous().span),
            });
        }

        let value = self.parse_expression()?;
        let semicolon =
            self.expect_kind(&TokenKind::Semicolon, "expected ';' after return statement")?;

        Ok(ReturnStatement {
            span: start.through(semicolon.span),
            value: Some(value),
        })
    }

    fn parse_while_statement(&mut self) -> Result<WhileStatement, ParseError> {
        let start = self.previous().span;

        self.expect_kind(&TokenKind::LeftParen, "expected '(' after 'while'")?;
        let condition = self.parse_expression()?;
        self.expect_kind(&TokenKind::RightParen, "expected ')' after while condition")?;
        let open_brace =
            self.expect_kind(&TokenKind::LeftBrace, "expected '{' after while condition")?;
        let body = self.parse_block_statement(open_brace)?;

        Ok(WhileStatement {
            span: start.through(body.span),
            condition,
            body,
        })
    }

    fn parse_block_statement(&mut self, open_brace: Token) -> Result<BlockStatement, ParseError> {
        let mut statements = Vec::new();

        while !self.check_simple(&TokenKind::RightBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }

        let close_brace = self.expect_kind(&TokenKind::RightBrace, "expected '}' to end block")?;

        Ok(BlockStatement {
            statements,
            span: open_brace.span.through(close_brace.span),
        })
    }

    fn parse_type_name(&mut self) -> Result<TypeName, ParseError> {
        let token = self.advance();

        let type_name = match token.kind {
            TokenKind::IntType => TypeName::Int,
            TokenKind::DoubleType => TypeName::Double,
            TokenKind::BoolType => TypeName::Bool,
            TokenKind::VoidType => TypeName::Void,
            _ => {
                return Err(ParseError::new(
                    self.source,
                    token.span,
                    format!("expected type annotation, found {}", token.kind.describe()),
                ));
            }
        };

        Ok(type_name)
    }

    fn parse_function_parameters(&mut self) -> Result<Vec<FunctionParameter>, ParseError> {
        let mut parameters = Vec::new();

        if self.check_simple(&TokenKind::RightParen) {
            return Ok(parameters);
        }

        loop {
            let name = self.expect_identifier("expected parameter name")?;
            self.expect_kind(&TokenKind::Colon, "expected ':' after parameter name")?;
            let type_annotation = self.parse_type_name()?;
            let span = name.span.through(self.previous().span);
            parameters.push(FunctionParameter {
                name,
                type_annotation,
                span,
            });

            if !self.match_simple(&TokenKind::Comma) {
                break;
            }
        }

        Ok(parameters)
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_equality()
    }

    fn parse_equality(&mut self) -> Result<Expression, ParseError> {
        let mut expression = self.parse_comparison()?;

        while let Some(operator) = self.match_binary_operator(&[
            (&TokenKind::EqualEqual, BinaryOperator::Equal),
            (&TokenKind::BangEqual, BinaryOperator::NotEqual),
        ]) {
            let right = self.parse_comparison()?;
            let span = expression.span().through(right.span());
            expression = Expression::Binary(BinaryExpression {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
                span,
            });
        }

        Ok(expression)
    }

    fn parse_comparison(&mut self) -> Result<Expression, ParseError> {
        let mut expression = self.parse_additive()?;

        while let Some(operator) = self.match_binary_operator(&[
            (&TokenKind::Less, BinaryOperator::Less),
            (&TokenKind::LessEqual, BinaryOperator::LessEqual),
            (&TokenKind::Greater, BinaryOperator::Greater),
            (&TokenKind::GreaterEqual, BinaryOperator::GreaterEqual),
        ]) {
            let right = self.parse_additive()?;
            let span = expression.span().through(right.span());
            expression = Expression::Binary(BinaryExpression {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
                span,
            });
        }

        Ok(expression)
    }

    fn parse_additive(&mut self) -> Result<Expression, ParseError> {
        let mut expression = self.parse_multiplicative()?;

        while let Some(operator) = self.match_binary_operator(&[
            (&TokenKind::Plus, BinaryOperator::Add),
            (&TokenKind::Minus, BinaryOperator::Subtract),
        ]) {
            let right = self.parse_multiplicative()?;
            let span = expression.span().through(right.span());
            expression = Expression::Binary(BinaryExpression {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
                span,
            });
        }

        Ok(expression)
    }

    fn parse_multiplicative(&mut self) -> Result<Expression, ParseError> {
        let mut expression = self.parse_unary()?;

        while let Some(operator) = self.match_binary_operator(&[
            (&TokenKind::Star, BinaryOperator::Multiply),
            (&TokenKind::Slash, BinaryOperator::Divide),
        ]) {
            let right = self.parse_unary()?;
            let span = expression.span().through(right.span());
            expression = Expression::Binary(BinaryExpression {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
                span,
            });
        }

        Ok(expression)
    }

    fn parse_unary(&mut self) -> Result<Expression, ParseError> {
        if self.match_simple(&TokenKind::Minus) {
            let operator_span = self.previous().span;
            let operand = self.parse_unary()?;
            let span = operator_span.through(operand.span());

            return Ok(Expression::Unary(UnaryExpression {
                operator: UnaryOperator::Negate,
                operand: Box::new(operand),
                span,
            }));
        }

        self.parse_call()
    }

    fn parse_call(&mut self) -> Result<Expression, ParseError> {
        let mut expression = self.parse_primary()?;

        while self.match_simple(&TokenKind::LeftParen) {
            let arguments = self.parse_call_arguments()?;
            let close_paren =
                self.expect_kind(&TokenKind::RightParen, "expected ')' after call arguments")?;
            let span = expression.span().through(close_paren.span);
            expression = Expression::Call(CallExpression {
                callee: Box::new(expression),
                arguments,
                span,
            });
        }

        Ok(expression)
    }

    fn parse_call_arguments(&mut self) -> Result<Vec<Expression>, ParseError> {
        let mut arguments = Vec::new();

        if self.check_simple(&TokenKind::RightParen) {
            return Ok(arguments);
        }

        loop {
            arguments.push(self.parse_expression()?);
            if !self.match_simple(&TokenKind::Comma) {
                break;
            }
        }

        Ok(arguments)
    }

    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        let token = self.advance();

        match token.kind {
            TokenKind::IntegerLiteral(value) => Ok(Expression::IntegerLiteral(IntegerLiteral {
                value,
                span: token.span,
            })),
            TokenKind::DoubleLiteral(value) => Ok(Expression::DoubleLiteral(DoubleLiteral {
                value,
                span: token.span,
            })),
            TokenKind::True => Ok(Expression::BoolLiteral(BoolLiteral {
                value: true,
                span: token.span,
            })),
            TokenKind::False => Ok(Expression::BoolLiteral(BoolLiteral {
                value: false,
                span: token.span,
            })),
            TokenKind::Identifier(name) => Ok(Expression::Identifier(Identifier {
                name,
                span: token.span,
            }))
            .and_then(|expression| self.parse_console_log_expression(expression)),
            TokenKind::LeftParen => {
                let expression = self.parse_expression()?;
                let close_paren =
                    self.expect_kind(&TokenKind::RightParen, "expected ')' after expression")?;

                Ok(expression.with_span(token.span.through(close_paren.span)))
            }
            _ => Err(ParseError::new(
                self.source,
                token.span,
                format!("expected expression, found {}", token.kind.describe()),
            )),
        }
    }

    fn parse_console_log_expression(
        &mut self,
        expression: Expression,
    ) -> Result<Expression, ParseError> {
        let Expression::Identifier(identifier) = &expression else {
            return Ok(expression);
        };

        if !self.check_simple(&TokenKind::Dot) {
            return Ok(expression);
        }

        if identifier.name != "console" {
            return Err(ParseError::new(
                self.source,
                self.peek().span,
                "only 'console.log(...)' property access is supported in v0",
            ));
        }

        let dot = self.advance();
        let property = self.advance();
        let TokenKind::Identifier(property_name) = property.kind else {
            return Err(ParseError::new(
                self.source,
                property.span,
                "expected property name after 'console.'",
            ));
        };

        if property_name != "log" {
            return Err(ParseError::new(
                self.source,
                property.span,
                format!("expected 'log' after 'console.', found '{property_name}'"),
            ));
        }

        if !self.match_simple(&TokenKind::LeftParen) {
            return Err(ParseError::new(
                self.source,
                self.peek().span,
                "expected '(' after 'console.log'",
            ));
        }

        let arguments = self.parse_call_arguments()?;
        let close_paren =
            self.expect_kind(&TokenKind::RightParen, "expected ')' after console.log arguments")?;

        Ok(Expression::ConsoleLog(ConsoleLogExpression {
            arguments,
            span: identifier.span.through(dot.span).through(close_paren.span),
        }))
    }

    fn match_binary_operator(
        &mut self,
        operators: &[(&TokenKind, BinaryOperator)],
    ) -> Option<BinaryOperator> {
        for (kind, operator) in operators {
            if self.match_simple(kind) {
                return Some(*operator);
            }
        }

        None
    }

    fn expect_identifier(&mut self, message: &str) -> Result<Identifier, ParseError> {
        let token = self.advance();

        if let TokenKind::Identifier(name) = token.kind {
            Ok(Identifier {
                name,
                span: token.span,
            })
        } else {
            Err(ParseError::new(self.source, token.span, message))
        }
    }

    fn expect_kind(&mut self, expected: &TokenKind, message: &str) -> Result<Token, ParseError> {
        if self.check_simple(expected) {
            Ok(self.advance())
        } else {
            let token = self.peek();
            Err(ParseError::new(
                self.source,
                token.span,
                format!("{message}, found {}", token.kind.describe()),
            ))
        }
    }

    fn is_assignment_start(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Identifier(_))
            && matches!(
                self.peek_next().map(|token| &token.kind),
                Some(TokenKind::Assign)
            )
    }

    fn is_expression_statement_start(&self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::Identifier(_)
                | TokenKind::IntegerLiteral(_)
                | TokenKind::DoubleLiteral(_)
                | TokenKind::True
                | TokenKind::False
                | TokenKind::LeftParen
                | TokenKind::Minus
        )
    }

    fn match_simple(&mut self, expected: &TokenKind) -> bool {
        if self.check_simple(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check_simple(&self, expected: &TokenKind) -> bool {
        mem::discriminant(&self.peek().kind) == mem::discriminant(expected)
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.current].clone();
        if !matches!(token.kind, TokenKind::EndOfFile) {
            self.current += 1;
        }
        token
    }

    fn previous(&self) -> Token {
        self.tokens[self.current.saturating_sub(1)].clone()
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn peek_next(&self) -> Option<&Token> {
        self.tokens.get(self.current + 1)
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek().kind, TokenKind::EndOfFile)
    }
}
