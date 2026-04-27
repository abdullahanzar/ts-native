use std::fmt;

use thiserror::Error;
use ts_native_parser::{
    Span,
    ast::{
        AssignmentStatement, BinaryOperator, BindingKind, BlockStatement, CallExpression,
        ConsoleLogExpression, Expression, ExpressionStatement, FunctionDeclaration,
        ReturnStatement, Statement, TypeName, VariableDeclaration, WhileStatement,
    },
};
use ts_native_types::{BindingInfo, BuiltinFunction, ResolvedSymbol, TypedProgram};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrModule {
    pub statements: Vec<IrStatement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrStatement {
    VariableDeclaration(IrVariableDeclaration),
    Assignment(IrAssignment),
    Expression(IrExpression),
    FunctionDeclaration(IrFunction),
    Return(Option<IrExpression>),
    While(IrWhileStatement),
    Block(IrBlock),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrBlock {
    pub statements: Vec<IrStatement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrFunction {
    pub name: String,
    pub parameters: Vec<IrParameter>,
    pub return_type: IrType,
    pub body: IrBlock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrParameter {
    pub name: String,
    pub type_name: IrType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrBindingKind {
    Const,
    Let,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrVariableDeclaration {
    pub kind: IrBindingKind,
    pub name: String,
    pub type_name: IrType,
    pub initializer: IrExpression,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrAssignment {
    pub target: String,
    pub type_name: IrType,
    pub value: IrExpression,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrWhileStatement {
    pub condition: IrExpression,
    pub body: IrBlock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrExpression {
    pub kind: IrExpressionKind,
    pub type_name: IrType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrExpressionKind {
    IntegerLiteral(i64),
    DoubleLiteral(String),
    BoolLiteral(bool),
    Variable(String),
    Unary {
        operator: IrUnaryOperator,
        operand: Box<IrExpression>,
    },
    Binary {
        left: Box<IrExpression>,
        operator: IrBinaryOperator,
        right: Box<IrExpression>,
    },
    Call {
        callee: String,
        arguments: Vec<IrExpression>,
    },
    Cast {
        expression: Box<IrExpression>,
        target_type: IrType,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrUnaryOperator {
    Negate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrBinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
    NotEqual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrType {
    Int,
    Double,
    Bool,
    Void,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct LoweringError {
    pub message: String,
}

impl LoweringError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    fn with_span(span: Span, message: impl Into<String>) -> Self {
        Self::new(format!(
            "{} at {}..{}",
            message.into(),
            span.start,
            span.end
        ))
    }
}

struct Lowerer<'a> {
    program: &'a TypedProgram,
    current_return_type: Option<IrType>,
}

/// TS-Native IR lowering entry point.
pub fn lower_to_tsn_ir(program: &TypedProgram) -> Result<IrModule, LoweringError> {
    Lowerer::new(program).lower_program()
}

impl<'a> Lowerer<'a> {
    fn new(program: &'a TypedProgram) -> Self {
        Self {
            program,
            current_return_type: None,
        }
    }

    fn lower_program(mut self) -> Result<IrModule, LoweringError> {
        Ok(IrModule {
            statements: self.lower_statements(&self.program.syntax.statements)?,
        })
    }

    fn lower_statements(
        &mut self,
        statements: &[Statement],
    ) -> Result<Vec<IrStatement>, LoweringError> {
        let mut lowered = Vec::with_capacity(statements.len());

        for statement in statements {
            lowered.push(self.lower_statement(statement)?);
        }

        Ok(lowered)
    }

    fn lower_statement(&mut self, statement: &Statement) -> Result<IrStatement, LoweringError> {
        match statement {
            Statement::VariableDeclaration(declaration) => self
                .lower_variable_declaration(declaration)
                .map(IrStatement::VariableDeclaration),
            Statement::Assignment(assignment) => self
                .lower_assignment(assignment)
                .map(IrStatement::Assignment),
            Statement::Expression(expression) => self
                .lower_expression_statement(expression)
                .map(IrStatement::Expression),
            Statement::FunctionDeclaration(function) => self
                .lower_function_declaration(function)
                .map(IrStatement::FunctionDeclaration),
            Statement::Return(return_statement) => self.lower_return_statement(return_statement),
            Statement::While(while_statement) => self
                .lower_while_statement(while_statement)
                .map(IrStatement::While),
            Statement::Block(block) => self.lower_block_statement(block).map(IrStatement::Block),
        }
    }

    fn lower_variable_declaration(
        &mut self,
        declaration: &VariableDeclaration,
    ) -> Result<IrVariableDeclaration, LoweringError> {
        let binding = self.binding(declaration.name.span)?;
        let mut initializer = self.lower_expression(&declaration.initializer)?;
        let type_name = ir_type(binding.type_name);
        initializer = coerce_expression(initializer, type_name)?;

        Ok(IrVariableDeclaration {
            kind: match declaration.kind {
                BindingKind::Const => IrBindingKind::Const,
                BindingKind::Let => IrBindingKind::Let,
            },
            name: declaration.name.name.clone(),
            type_name,
            initializer,
        })
    }

    fn lower_assignment(
        &mut self,
        assignment: &AssignmentStatement,
    ) -> Result<IrAssignment, LoweringError> {
        let ResolvedSymbol::Variable(binding) = self.resolved_symbol(assignment.target.span)?
        else {
            return Err(LoweringError::with_span(
                assignment.target.span,
                "lowering invariant violated: assignment target must resolve to a variable",
            ));
        };

        let target_type = ir_type(binding.type_name);
        let value = coerce_expression(self.lower_expression(&assignment.value)?, target_type)?;

        Ok(IrAssignment {
            target: assignment.target.name.clone(),
            type_name: target_type,
            value,
        })
    }

    fn lower_expression_statement(
        &mut self,
        expression: &ExpressionStatement,
    ) -> Result<IrExpression, LoweringError> {
        self.lower_expression(&expression.expression)
    }

    fn lower_function_declaration(
        &mut self,
        function: &FunctionDeclaration,
    ) -> Result<IrFunction, LoweringError> {
        let previous_return_type = self.current_return_type;
        self.current_return_type = Some(ir_type(function.return_type));

        let mut parameters = Vec::with_capacity(function.parameters.len());
        for parameter in &function.parameters {
            let parameter_type = ir_type(self.binding(parameter.name.span)?.type_name);
            parameters.push(IrParameter {
                name: parameter.name.name.clone(),
                type_name: parameter_type,
            });
        }

        let body = IrBlock {
            statements: self.lower_statements(&function.body.statements)?,
        };

        self.current_return_type = previous_return_type;

        Ok(IrFunction {
            name: function.name.name.clone(),
            parameters,
            return_type: ir_type(function.return_type),
            body,
        })
    }

    fn lower_return_statement(
        &mut self,
        return_statement: &ReturnStatement,
    ) -> Result<IrStatement, LoweringError> {
        let expected_return_type = self.current_return_type.ok_or_else(|| {
            LoweringError::with_span(
                return_statement.span,
                "lowering invariant violated: return statement outside function",
            )
        })?;

        let value = match &return_statement.value {
            Some(expression) => Some(coerce_expression(
                self.lower_expression(expression)?,
                expected_return_type,
            )?),
            None => None,
        };

        Ok(IrStatement::Return(value))
    }

    fn lower_while_statement(
        &mut self,
        while_statement: &WhileStatement,
    ) -> Result<IrWhileStatement, LoweringError> {
        let condition = self.lower_expression(&while_statement.condition)?;
        if condition.type_name != IrType::Bool {
            return Err(LoweringError::with_span(
                while_statement.condition.span(),
                "lowering invariant violated: while condition must be bool",
            ));
        }

        Ok(IrWhileStatement {
            condition,
            body: self.lower_block_statement(&while_statement.body)?,
        })
    }

    fn lower_block_statement(&mut self, block: &BlockStatement) -> Result<IrBlock, LoweringError> {
        Ok(IrBlock {
            statements: self.lower_statements(&block.statements)?,
        })
    }

    fn lower_expression(&mut self, expression: &Expression) -> Result<IrExpression, LoweringError> {
        let semantic_type = self.expression_type(expression.span())?;

        match expression {
            Expression::IntegerLiteral(literal) => Ok(IrExpression {
                kind: IrExpressionKind::IntegerLiteral(literal.value),
                type_name: semantic_type,
            }),
            Expression::DoubleLiteral(literal) => Ok(IrExpression {
                kind: IrExpressionKind::DoubleLiteral(format_double_literal(literal.value)),
                type_name: semantic_type,
            }),
            Expression::BoolLiteral(literal) => Ok(IrExpression {
                kind: IrExpressionKind::BoolLiteral(literal.value),
                type_name: semantic_type,
            }),
            Expression::Identifier(identifier) => {
                let ResolvedSymbol::Variable(binding) = self.resolved_symbol(identifier.span)?
                else {
                    return Err(LoweringError::with_span(
                        identifier.span,
                        "lowering invariant violated: identifier expression must resolve to a variable",
                    ));
                };

                Ok(IrExpression {
                    kind: IrExpressionKind::Variable(identifier.name.clone()),
                    type_name: ir_type(binding.type_name),
                })
            }
            Expression::Unary(unary) => {
                let operand = self.lower_expression(&unary.operand)?;
                if !is_numeric(operand.type_name) || semantic_type != operand.type_name {
                    return Err(LoweringError::with_span(
                        unary.span,
                        "lowering invariant violated: unary negate requires numeric operand",
                    ));
                }

                Ok(IrExpression {
                    type_name: semantic_type,
                    kind: IrExpressionKind::Unary {
                        operator: IrUnaryOperator::Negate,
                        operand: Box::new(operand),
                    },
                })
            }
            Expression::Binary(binary) => self.lower_binary_expression(binary),
            Expression::Call(call) => self.lower_call_expression(call),
            Expression::ConsoleLog(console_log) => {
                self.lower_console_log_expression(console_log)
            }
        }
    }

    fn lower_binary_expression(
        &mut self,
        binary: &ts_native_parser::ast::BinaryExpression,
    ) -> Result<IrExpression, LoweringError> {
        let result_type = self.expression_type(binary.span)?;
        let left = self.lower_expression(&binary.left)?;
        let right = self.lower_expression(&binary.right)?;
        let operator = ir_binary_operator(binary.operator);

        match binary.operator {
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide => {
                if !is_numeric(left.type_name) || !is_numeric(right.type_name) {
                    return Err(LoweringError::with_span(
                        binary.span,
                        "lowering invariant violated: arithmetic operands must be numeric",
                    ));
                }

                Ok(IrExpression {
                    type_name: result_type,
                    kind: IrExpressionKind::Binary {
                        left: Box::new(coerce_expression(left, result_type)?),
                        operator,
                        right: Box::new(coerce_expression(right, result_type)?),
                    },
                })
            }
            BinaryOperator::Less
            | BinaryOperator::LessEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual => {
                if !is_numeric(left.type_name) || !is_numeric(right.type_name) {
                    return Err(LoweringError::with_span(
                        binary.span,
                        "lowering invariant violated: comparison operands must be numeric",
                    ));
                }

                let operand_type = promote_numeric(left.type_name, right.type_name);
                Ok(IrExpression {
                    type_name: result_type,
                    kind: IrExpressionKind::Binary {
                        left: Box::new(coerce_expression(left, operand_type)?),
                        operator,
                        right: Box::new(coerce_expression(right, operand_type)?),
                    },
                })
            }
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                let common_type = if left.type_name == right.type_name {
                    left.type_name
                } else if is_numeric(left.type_name) && is_numeric(right.type_name) {
                    promote_numeric(left.type_name, right.type_name)
                } else {
                    return Err(LoweringError::with_span(
                        binary.span,
                        "lowering invariant violated: equality operands are incompatible",
                    ));
                };

                Ok(IrExpression {
                    type_name: result_type,
                    kind: IrExpressionKind::Binary {
                        left: Box::new(coerce_expression(left, common_type)?),
                        operator,
                        right: Box::new(coerce_expression(right, common_type)?),
                    },
                })
            }
        }
    }

    fn lower_call_expression(
        &mut self,
        call: &CallExpression,
    ) -> Result<IrExpression, LoweringError> {
        let Expression::Identifier(identifier) = call.callee.as_ref() else {
            return Err(LoweringError::with_span(
                call.span,
                "lowering invariant violated: only named functions can be called in v0",
            ));
        };

        let (callee, parameter_types) = match self.resolved_symbol(identifier.span)? {
            ResolvedSymbol::Function(function) => {
                (function.name.clone(), function.parameters.clone())
            }
            ResolvedSymbol::BuiltinFunction(builtin) => builtin_call_signature(*builtin),
            ResolvedSymbol::Variable(_) => {
                return Err(LoweringError::with_span(
                    identifier.span,
                    "lowering invariant violated: call target must resolve to a function",
                ));
            }
        };

        let mut arguments = Vec::with_capacity(call.arguments.len());
        for (argument, expected_type) in call.arguments.iter().zip(&parameter_types) {
            arguments.push(coerce_expression(
                self.lower_expression(argument)?,
                ir_type(*expected_type),
            )?);
        }

        Ok(IrExpression {
            kind: IrExpressionKind::Call {
                callee,
                arguments,
            },
            type_name: self.expression_type(call.span)?,
        })
    }

    fn lower_console_log_expression(
        &mut self,
        console_log: &ConsoleLogExpression,
    ) -> Result<IrExpression, LoweringError> {
        let [argument] = console_log.arguments.as_slice() else {
            return Err(LoweringError::with_span(
                console_log.span,
                "lowering invariant violated: console.log expects exactly one argument",
            ));
        };

        let argument = self.lower_expression(argument)?;
        let callee = match argument.type_name {
            IrType::Int => "printInt",
            IrType::Double => "printDouble",
            IrType::Bool => "printBool",
            IrType::Void => {
                return Err(LoweringError::with_span(
                    console_log.span,
                    "lowering invariant violated: console.log cannot print void values",
                ));
            }
        };

        Ok(IrExpression {
            kind: IrExpressionKind::Call {
                callee: callee.to_owned(),
                arguments: vec![argument],
            },
            type_name: self.expression_type(console_log.span)?,
        })
    }

    fn expression_type(&self, span: Span) -> Result<IrType, LoweringError> {
        self.program
            .expression_type(span)
            .map(ir_type)
            .ok_or_else(|| {
                LoweringError::with_span(
                    span,
                    "lowering invariant violated: missing expression type",
                )
            })
    }

    fn binding(&self, span: Span) -> Result<BindingInfo, LoweringError> {
        self.program.binding(span).copied().ok_or_else(|| {
            LoweringError::with_span(span, "lowering invariant violated: missing binding info")
        })
    }

    fn resolved_symbol(&self, span: Span) -> Result<&ResolvedSymbol, LoweringError> {
        self.program.resolved_symbol(span).ok_or_else(|| {
            LoweringError::with_span(
                span,
                "lowering invariant violated: missing symbol resolution",
            )
        })
    }
}

fn coerce_expression(
    expression: IrExpression,
    target_type: IrType,
) -> Result<IrExpression, LoweringError> {
    if expression.type_name == target_type {
        Ok(expression)
    } else if expression.type_name == IrType::Int && target_type == IrType::Double {
        Ok(IrExpression {
            kind: IrExpressionKind::Cast {
                expression: Box::new(expression),
                target_type,
            },
            type_name: target_type,
        })
    } else {
        Err(LoweringError::new(format!(
            "lowering invariant violated: cannot coerce {} to {}",
            expression.type_name, target_type
        )))
    }
}

fn is_numeric(type_name: IrType) -> bool {
    matches!(type_name, IrType::Int | IrType::Double)
}

fn promote_numeric(left: IrType, right: IrType) -> IrType {
    if left == IrType::Double || right == IrType::Double {
        IrType::Double
    } else {
        IrType::Int
    }
}

fn ir_type(type_name: TypeName) -> IrType {
    match type_name {
        TypeName::Int => IrType::Int,
        TypeName::Double => IrType::Double,
        TypeName::Bool => IrType::Bool,
        TypeName::Void => IrType::Void,
    }
}

fn ir_binary_operator(operator: BinaryOperator) -> IrBinaryOperator {
    match operator {
        BinaryOperator::Add => IrBinaryOperator::Add,
        BinaryOperator::Subtract => IrBinaryOperator::Subtract,
        BinaryOperator::Multiply => IrBinaryOperator::Multiply,
        BinaryOperator::Divide => IrBinaryOperator::Divide,
        BinaryOperator::Less => IrBinaryOperator::Less,
        BinaryOperator::LessEqual => IrBinaryOperator::LessEqual,
        BinaryOperator::Greater => IrBinaryOperator::Greater,
        BinaryOperator::GreaterEqual => IrBinaryOperator::GreaterEqual,
        BinaryOperator::Equal => IrBinaryOperator::Equal,
        BinaryOperator::NotEqual => IrBinaryOperator::NotEqual,
    }
}

fn format_double_literal(value: f64) -> String {
    let mut text = value.to_string();
    if !text.contains(['.', 'e', 'E']) {
        text.push_str(".0");
    }
    text
}

fn builtin_call_signature(builtin: BuiltinFunction) -> (String, Vec<TypeName>) {
    let signature = builtin.signature();
    (signature.name, signature.parameters)
}

impl fmt::Display for IrModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "module {{")?;
        for (index, statement) in self.statements.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            write_statement(f, statement, 1)?;
        }
        write!(f, "}}")
    }
}

impl fmt::Display for IrType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int => write!(f, "int"),
            Self::Double => write!(f, "double"),
            Self::Bool => write!(f, "bool"),
            Self::Void => write!(f, "void"),
        }
    }
}

fn write_statement(
    f: &mut fmt::Formatter<'_>,
    statement: &IrStatement,
    indent: usize,
) -> fmt::Result {
    match statement {
        IrStatement::VariableDeclaration(declaration) => {
            indent_line(f, indent)?;
            let keyword = match declaration.kind {
                IrBindingKind::Const => "const",
                IrBindingKind::Let => "let",
            };
            writeln!(
                f,
                "{keyword} {}: {} = {};",
                declaration.name,
                declaration.type_name,
                format_expression(&declaration.initializer)
            )
        }
        IrStatement::Assignment(assignment) => {
            indent_line(f, indent)?;
            writeln!(
                f,
                "{} = {};",
                assignment.target,
                format_expression(&assignment.value)
            )
        }
        IrStatement::Expression(expression) => {
            indent_line(f, indent)?;
            writeln!(f, "{};", format_expression(expression))
        }
        IrStatement::FunctionDeclaration(function) => {
            indent_line(f, indent)?;
            write!(f, "function {}(", function.name)?;
            for (index, parameter) in function.parameters.iter().enumerate() {
                if index > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}: {}", parameter.name, parameter.type_name)?;
            }
            writeln!(f, "): {} {{", function.return_type)?;
            write_block_contents(f, &function.body, indent + 1)?;
            indent_line(f, indent)?;
            writeln!(f, "}}")
        }
        IrStatement::Return(value) => {
            indent_line(f, indent)?;
            match value {
                Some(value) => writeln!(f, "return {};", format_expression(value)),
                None => writeln!(f, "return;"),
            }
        }
        IrStatement::While(while_statement) => {
            indent_line(f, indent)?;
            writeln!(
                f,
                "while ({}) {{",
                format_expression(&while_statement.condition)
            )?;
            write_block_contents(f, &while_statement.body, indent + 1)?;
            indent_line(f, indent)?;
            writeln!(f, "}}")
        }
        IrStatement::Block(block) => {
            indent_line(f, indent)?;
            writeln!(f, "{{")?;
            write_block_contents(f, block, indent + 1)?;
            indent_line(f, indent)?;
            writeln!(f, "}}")
        }
    }
}

fn write_block_contents(f: &mut fmt::Formatter<'_>, block: &IrBlock, indent: usize) -> fmt::Result {
    for statement in &block.statements {
        write_statement(f, statement, indent)?;
    }
    Ok(())
}

fn indent_line(f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
    for _ in 0..indent {
        write!(f, "  ")?;
    }
    Ok(())
}

fn format_expression(expression: &IrExpression) -> String {
    match &expression.kind {
        IrExpressionKind::IntegerLiteral(value) => value.to_string(),
        IrExpressionKind::DoubleLiteral(value) => value.clone(),
        IrExpressionKind::BoolLiteral(value) => value.to_string(),
        IrExpressionKind::Variable(name) => name.clone(),
        IrExpressionKind::Unary { operator, operand } => match operator {
            IrUnaryOperator::Negate => format!("(-{})", format_expression(operand)),
        },
        IrExpressionKind::Binary {
            left,
            operator,
            right,
        } => format!(
            "({} {} {})",
            format_expression(left),
            format_binary_operator(*operator),
            format_expression(right)
        ),
        IrExpressionKind::Call { callee, arguments } => {
            let arguments = arguments
                .iter()
                .map(format_expression)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{callee}({arguments})")
        }
        IrExpressionKind::Cast {
            expression,
            target_type,
        } => format!("cast<{target_type}>({})", format_expression(expression)),
    }
}

fn format_binary_operator(operator: IrBinaryOperator) -> &'static str {
    match operator {
        IrBinaryOperator::Add => "+",
        IrBinaryOperator::Subtract => "-",
        IrBinaryOperator::Multiply => "*",
        IrBinaryOperator::Divide => "/",
        IrBinaryOperator::Less => "<",
        IrBinaryOperator::LessEqual => "<=",
        IrBinaryOperator::Greater => ">",
        IrBinaryOperator::GreaterEqual => ">=",
        IrBinaryOperator::Equal => "==",
        IrBinaryOperator::NotEqual => "!=",
    }
}
