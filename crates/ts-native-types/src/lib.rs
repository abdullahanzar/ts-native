use std::collections::HashMap;

use thiserror::Error;
use ts_native_parser::{
    Span,
    ast::{
        AssignmentStatement, BinaryOperator, BindingKind, BlockStatement, CallExpression,
        ConsoleLogExpression, Expression, ExpressionStatement, FunctionDeclaration, Program,
        ReturnStatement, Statement, TypeName, UnaryOperator, VariableDeclaration, WhileStatement,
    },
};

#[derive(Debug, Clone, PartialEq)]
pub struct TypedProgram {
    pub syntax: Program,
    pub functions: Vec<FunctionSignature>,
    pub semantics: SemanticModel,
}

impl TypedProgram {
    #[must_use]
    pub fn expression_type(&self, span: Span) -> Option<TypeName> {
        self.semantics.expression_types.get(&span).copied()
    }

    #[must_use]
    pub fn binding(&self, span: Span) -> Option<&BindingInfo> {
        self.semantics.bindings.get(&span)
    }

    #[must_use]
    pub fn resolved_symbol(&self, span: Span) -> Option<&ResolvedSymbol> {
        self.semantics.references.get(&span)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SemanticModel {
    pub expression_types: HashMap<Span, TypeName>,
    pub bindings: HashMap<Span, BindingInfo>,
    pub references: HashMap<Span, ResolvedSymbol>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindingInfo {
    pub type_name: TypeName,
    pub mutable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedSymbol {
    Variable(BindingInfo),
    Function(FunctionSignature),
    BuiltinFunction(BuiltinFunction),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSignature {
    pub name: String,
    pub parameters: Vec<TypeName>,
    pub return_type: TypeName,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinFunction {
    PrintInt,
    PrintDouble,
    PrintBool,
}

impl BuiltinFunction {
    const ALL: [Self; 3] = [Self::PrintInt, Self::PrintDouble, Self::PrintBool];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::PrintInt => "printInt",
            Self::PrintDouble => "printDouble",
            Self::PrintBool => "printBool",
        }
    }

    #[must_use]
    pub fn signature(self) -> FunctionSignature {
        match self {
            Self::PrintInt => FunctionSignature {
                name: self.name().to_owned(),
                parameters: vec![TypeName::Int],
                return_type: TypeName::Void,
            },
            Self::PrintDouble => FunctionSignature {
                name: self.name().to_owned(),
                parameters: vec![TypeName::Double],
                return_type: TypeName::Void,
            },
            Self::PrintBool => FunctionSignature {
                name: self.name().to_owned(),
                parameters: vec![TypeName::Bool],
                return_type: TypeName::Void,
            },
        }
    }

    #[must_use]
    pub fn lookup(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|builtin| builtin.name() == name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message} at line {line}, column {column}")]
pub struct TypeError {
    pub message: String,
    pub span: Span,
    pub line: usize,
    pub column: usize,
}

impl TypeError {
    #[must_use]
    pub fn new(source: &str, span: Span, message: impl Into<String>) -> Self {
        let (line, column) = line_column(source, span.start);

        Self {
            message: message.into(),
            span,
            line,
            column,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Symbol {
    Variable(VariableSymbol),
    Function(FunctionSignature),
    BuiltinFunction(BuiltinFunction),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VariableSymbol {
    type_name: TypeName,
    mutable: bool,
}

struct TypeChecker<'a> {
    source: &'a str,
    scopes: Vec<HashMap<String, Symbol>>,
    current_return_type: Option<TypeName>,
    functions: Vec<FunctionSignature>,
    semantics: SemanticModel,
}

/// Type-checker entry point for validated syntax trees.
pub fn type_check(program: &Program, source: &str) -> Result<TypedProgram, TypeError> {
    TypeChecker::new(source).check_program(program)
}

impl<'a> TypeChecker<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            scopes: Vec::new(),
            current_return_type: None,
            functions: Vec::new(),
            semantics: SemanticModel::default(),
        }
    }

    fn check_program(mut self, program: &Program) -> Result<TypedProgram, TypeError> {
        self.push_scope();
        self.declare_builtin_functions()?;
        self.predeclare_functions(&program.statements)?;
        self.check_statements(&program.statements)?;

        Ok(TypedProgram {
            syntax: program.clone(),
            functions: self.functions,
            semantics: self.semantics,
        })
    }

    fn check_statements(&mut self, statements: &[Statement]) -> Result<(), TypeError> {
        for statement in statements {
            self.check_statement(statement)?;
        }

        Ok(())
    }

    fn check_statement(&mut self, statement: &Statement) -> Result<(), TypeError> {
        match statement {
            Statement::VariableDeclaration(declaration) => {
                self.check_variable_declaration(declaration)
            }
            Statement::Assignment(assignment) => self.check_assignment(assignment),
            Statement::Expression(expression) => self.check_expression_statement(expression),
            Statement::FunctionDeclaration(function) => self.check_function_declaration(function),
            Statement::Return(return_statement) => self.check_return_statement(return_statement),
            Statement::While(while_statement) => self.check_while_statement(while_statement),
            Statement::Block(block) => self.check_block(block),
        }
    }

    fn check_variable_declaration(
        &mut self,
        declaration: &VariableDeclaration,
    ) -> Result<(), TypeError> {
        if let Some(annotation) = declaration.type_annotation {
            self.ensure_non_void_type(
                annotation,
                declaration.name.span,
                "variables cannot have type 'void'",
            )?;
        }

        let initializer_type = self.check_expression(&declaration.initializer)?;
        let binding_type = if let Some(annotation) = declaration.type_annotation {
            self.expect_assignable(
                initializer_type,
                annotation,
                declaration.initializer.span(),
                format!(
                    "cannot assign value of type {} to variable '{}' of type {}",
                    describe_type(initializer_type),
                    declaration.name.name,
                    describe_type(annotation)
                ),
            )?;
            annotation
        } else {
            self.ensure_non_void_type(
                initializer_type,
                declaration.initializer.span(),
                "variables cannot be inferred as type 'void'",
            )?;
            initializer_type
        };

        self.declare_symbol(
            declaration.name.name.clone(),
            Symbol::Variable(VariableSymbol {
                type_name: binding_type,
                mutable: matches!(declaration.kind, BindingKind::Let),
            }),
            declaration.name.span,
        )
    }

    fn check_assignment(&mut self, assignment: &AssignmentStatement) -> Result<(), TypeError> {
        let symbol = self.lookup_symbol(&assignment.target.name, assignment.target.span)?;

        match symbol {
            Symbol::Variable(variable) => {
                if !variable.mutable {
                    return Err(TypeError::new(
                        self.source,
                        assignment.target.span,
                        format!(
                            "cannot assign to immutable binding '{}'",
                            assignment.target.name
                        ),
                    ));
                }

                let value_type = self.check_expression(&assignment.value)?;
                self.expect_assignable(
                    value_type,
                    variable.type_name,
                    assignment.value.span(),
                    format!(
                        "cannot assign value of type {} to binding '{}' of type {}",
                        describe_type(value_type),
                        assignment.target.name,
                        describe_type(variable.type_name)
                    ),
                )
            }
            Symbol::Function(_) | Symbol::BuiltinFunction(_) => Err(TypeError::new(
                self.source,
                assignment.target.span,
                format!("cannot assign to function '{}'", assignment.target.name),
            )),
        }
    }

    fn check_expression_statement(
        &mut self,
        expression: &ExpressionStatement,
    ) -> Result<(), TypeError> {
        match &expression.expression {
            Expression::Call(_) | Expression::ConsoleLog(_) => {
                self.check_expression(&expression.expression)?;
                Ok(())
            }
            _ => Err(TypeError::new(
                self.source,
                expression.span,
                "only call expressions may appear as standalone statements",
            )),
        }
    }

    fn check_function_declaration(
        &mut self,
        function: &FunctionDeclaration,
    ) -> Result<(), TypeError> {
        let previous_return_type = self.current_return_type;
        self.current_return_type = Some(function.return_type);
        self.push_scope();
        self.predeclare_functions(&function.body.statements)?;

        for parameter in &function.parameters {
            self.ensure_non_void_type(
                parameter.type_annotation,
                parameter.span,
                "function parameters cannot have type 'void'",
            )?;

            self.declare_symbol(
                parameter.name.name.clone(),
                Symbol::Variable(VariableSymbol {
                    type_name: parameter.type_annotation,
                    mutable: true,
                }),
                parameter.name.span,
            )?;
        }

        self.check_statements(&function.body.statements)?;

        if function.return_type != TypeName::Void && !contains_return_statement(&function.body) {
            self.pop_scope();
            self.current_return_type = previous_return_type;
            return Err(TypeError::new(
                self.source,
                function.name.span,
                format!(
                    "function '{}' declares return type {} but has no return statement",
                    function.name.name,
                    describe_type(function.return_type)
                ),
            ));
        }

        self.pop_scope();
        self.current_return_type = previous_return_type;
        Ok(())
    }

    fn check_return_statement(
        &mut self,
        return_statement: &ReturnStatement,
    ) -> Result<(), TypeError> {
        let Some(expected_return_type) = self.current_return_type else {
            return Err(TypeError::new(
                self.source,
                return_statement.span,
                "return statements are only valid inside functions",
            ));
        };

        match (expected_return_type, &return_statement.value) {
            (TypeName::Void, None) => Ok(()),
            (TypeName::Void, Some(value)) => Err(TypeError::new(
                self.source,
                value.span(),
                "void functions cannot return a value",
            )),
            (expected, Some(value)) => {
                let actual = self.check_expression(value)?;
                self.expect_assignable(
                    actual,
                    expected,
                    value.span(),
                    format!(
                        "cannot return value of type {} from function returning {}",
                        describe_type(actual),
                        describe_type(expected)
                    ),
                )
            }
            (expected, None) => Err(TypeError::new(
                self.source,
                return_statement.span,
                format!(
                    "function returning {} must return a value",
                    describe_type(expected)
                ),
            )),
        }
    }

    fn check_while_statement(&mut self, while_statement: &WhileStatement) -> Result<(), TypeError> {
        let condition_type = self.check_expression(&while_statement.condition)?;
        if condition_type != TypeName::Bool {
            return Err(TypeError::new(
                self.source,
                while_statement.condition.span(),
                format!(
                    "while condition must have type bool, found {}",
                    describe_type(condition_type)
                ),
            ));
        }

        self.check_block(&while_statement.body)
    }

    fn check_block(&mut self, block: &BlockStatement) -> Result<(), TypeError> {
        self.push_scope();
        self.predeclare_functions(&block.statements)?;
        let result = self.check_statements(&block.statements);
        self.pop_scope();
        result
    }

    fn check_expression(&mut self, expression: &Expression) -> Result<TypeName, TypeError> {
        match expression {
            Expression::IntegerLiteral(literal) => {
                Ok(self.record_expression_type(literal.span, TypeName::Int))
            }
            Expression::DoubleLiteral(literal) => {
                Ok(self.record_expression_type(literal.span, TypeName::Double))
            }
            Expression::BoolLiteral(literal) => {
                Ok(self.record_expression_type(literal.span, TypeName::Bool))
            }
            Expression::Identifier(identifier) => {
                match self.lookup_symbol(&identifier.name, identifier.span)? {
                    Symbol::Variable(variable) => {
                        Ok(self.record_expression_type(identifier.span, variable.type_name))
                    }
                    Symbol::Function(_) | Symbol::BuiltinFunction(_) => Err(TypeError::new(
                        self.source,
                        identifier.span,
                        format!("function '{}' cannot be used as a value", identifier.name),
                    )),
                }
            }
            Expression::Unary(unary) => {
                let operand_type = self.check_expression(&unary.operand)?;
                match unary.operator {
                    UnaryOperator::Negate => {
                        if is_numeric(operand_type) {
                            Ok(self.record_expression_type(unary.span, operand_type))
                        } else {
                            Err(TypeError::new(
                                self.source,
                                unary.span,
                                format!(
                                    "unary '-' requires a numeric operand, found {}",
                                    describe_type(operand_type)
                                ),
                            ))
                        }
                    }
                }
            }
            Expression::Binary(binary) => self.check_binary_expression(
                binary.operator,
                &binary.left,
                &binary.right,
                binary.span,
            ),
            Expression::Call(call) => self.check_call_expression(call),
            Expression::ConsoleLog(console_log) => self.check_console_log_expression(console_log),
        }
    }

    fn check_binary_expression(
        &mut self,
        operator: BinaryOperator,
        left: &Expression,
        right: &Expression,
        span: Span,
    ) -> Result<TypeName, TypeError> {
        let left_type = self.check_expression(left)?;
        let right_type = self.check_expression(right)?;

        match operator {
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide => {
                if !is_numeric(left_type) || !is_numeric(right_type) {
                    return Err(TypeError::new(
                        self.source,
                        span,
                        format!(
                            "arithmetic operator requires numeric operands, found {} and {}",
                            describe_type(left_type),
                            describe_type(right_type)
                        ),
                    ));
                }

                Ok(self.record_expression_type(span, promote_numeric(left_type, right_type)))
            }
            BinaryOperator::Less
            | BinaryOperator::LessEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual => {
                if !is_numeric(left_type) || !is_numeric(right_type) {
                    return Err(TypeError::new(
                        self.source,
                        span,
                        format!(
                            "comparison operator requires numeric operands, found {} and {}",
                            describe_type(left_type),
                            describe_type(right_type)
                        ),
                    ));
                }

                Ok(self.record_expression_type(span, TypeName::Bool))
            }
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                if types_compatible(left_type, right_type) {
                    Ok(self.record_expression_type(span, TypeName::Bool))
                } else {
                    Err(TypeError::new(
                        self.source,
                        span,
                        format!(
                            "equality operator requires comparable operands, found {} and {}",
                            describe_type(left_type),
                            describe_type(right_type)
                        ),
                    ))
                }
            }
        }
    }

    fn check_call_expression(&mut self, call: &CallExpression) -> Result<TypeName, TypeError> {
        let Expression::Identifier(identifier) = call.callee.as_ref() else {
            return Err(TypeError::new(
                self.source,
                call.callee.span(),
                "only named functions can be called in v0",
            ));
        };

        let symbol = self.lookup_symbol(&identifier.name, identifier.span)?;
        let signature = match symbol {
            Symbol::Function(signature) => signature,
            Symbol::BuiltinFunction(builtin) => builtin.signature(),
            Symbol::Variable(_) => {
                return Err(TypeError::new(
                    self.source,
                    identifier.span,
                    format!("'{}' is not a function", identifier.name),
                ));
            }
        };

        if call.arguments.len() != signature.parameters.len() {
            return Err(TypeError::new(
                self.source,
                call.span,
                format!(
                    "function '{}' expects {} argument(s), found {}",
                    identifier.name,
                    signature.parameters.len(),
                    call.arguments.len()
                ),
            ));
        }

        for (argument, expected) in call.arguments.iter().zip(&signature.parameters) {
            let actual = self.check_expression(argument)?;
            self.expect_assignable(
                actual,
                *expected,
                argument.span(),
                format!(
                    "cannot pass value of type {} to parameter of type {}",
                    describe_type(actual),
                    describe_type(*expected)
                ),
            )?;
        }

        Ok(self.record_expression_type(call.span, signature.return_type))
    }

    fn check_console_log_expression(
        &mut self,
        console_log: &ConsoleLogExpression,
    ) -> Result<TypeName, TypeError> {
        if console_log.arguments.len() != 1 {
            return Err(TypeError::new(
                self.source,
                console_log.span,
                format!(
                    "console.log expects 1 argument(s), found {}",
                    console_log.arguments.len()
                ),
            ));
        }

        let argument = &console_log.arguments[0];
        match self.check_expression(argument)? {
            TypeName::Int | TypeName::Double | TypeName::Bool => {
                Ok(self.record_expression_type(console_log.span, TypeName::Void))
            }
            TypeName::Void => Err(TypeError::new(
                self.source,
                argument.span(),
                "console.log does not accept values of type void",
            )),
        }
    }

    fn declare_builtin_functions(&mut self) -> Result<(), TypeError> {
        for builtin in BuiltinFunction::ALL {
            self.declare_builtin_symbol(builtin)?;
        }

        Ok(())
    }

    fn predeclare_functions(&mut self, statements: &[Statement]) -> Result<(), TypeError> {
        for statement in statements {
            if let Statement::FunctionDeclaration(function) = statement {
                for parameter in &function.parameters {
                    self.ensure_non_void_type(
                        parameter.type_annotation,
                        parameter.span,
                        "function parameters cannot have type 'void'",
                    )?;
                }

                let signature = FunctionSignature {
                    name: function.name.name.clone(),
                    parameters: function
                        .parameters
                        .iter()
                        .map(|parameter| parameter.type_annotation)
                        .collect(),
                    return_type: function.return_type,
                };

                self.declare_symbol(
                    signature.name.clone(),
                    Symbol::Function(signature.clone()),
                    function.name.span,
                )?;
                self.functions.push(signature);
            }
        }

        Ok(())
    }

    fn ensure_non_void_type(
        &self,
        type_name: TypeName,
        span: Span,
        message: &str,
    ) -> Result<(), TypeError> {
        if type_name == TypeName::Void {
            Err(TypeError::new(self.source, span, message))
        } else {
            Ok(())
        }
    }

    fn expect_assignable(
        &self,
        actual: TypeName,
        expected: TypeName,
        span: Span,
        message: String,
    ) -> Result<(), TypeError> {
        if is_assignable(actual, expected) {
            Ok(())
        } else {
            Err(TypeError::new(self.source, span, message))
        }
    }

    fn declare_symbol(
        &mut self,
        name: String,
        symbol: Symbol,
        span: Span,
    ) -> Result<(), TypeError> {
        if BuiltinFunction::lookup(&name).is_some() {
            return Err(TypeError::new(
                self.source,
                span,
                format!("'{}' is a reserved builtin function name", name),
            ));
        }

        let scope = self
            .scopes
            .last_mut()
            .expect("type checker must always have an active scope");

        if scope.contains_key(&name) {
            return Err(TypeError::new(
                self.source,
                span,
                format!("redeclaration of '{}' in the same scope", name),
            ));
        }

        if let Symbol::Variable(variable) = &symbol {
            self.semantics.bindings.insert(
                span,
                BindingInfo {
                    type_name: variable.type_name,
                    mutable: variable.mutable,
                },
            );
        }

        scope.insert(name, symbol);
        Ok(())
    }

    fn declare_builtin_symbol(&mut self, builtin: BuiltinFunction) -> Result<(), TypeError> {
        let scope = self
            .scopes
            .last_mut()
            .expect("type checker must always have an active scope");
        scope.insert(builtin.name().to_owned(), Symbol::BuiltinFunction(builtin));
        Ok(())
    }

    fn lookup_symbol(&mut self, name: &str, span: Span) -> Result<Symbol, TypeError> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                let symbol = symbol.clone();
                self.record_reference(span, &symbol);
                return Ok(symbol);
            }
        }

        Err(TypeError::new(
            self.source,
            span,
            format!("use of undeclared identifier '{}'", name),
        ))
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn record_expression_type(&mut self, span: Span, type_name: TypeName) -> TypeName {
        self.semantics.expression_types.insert(span, type_name);
        type_name
    }

    fn record_reference(&mut self, span: Span, symbol: &Symbol) {
        let resolved = match symbol {
            Symbol::Variable(variable) => ResolvedSymbol::Variable(BindingInfo {
                type_name: variable.type_name,
                mutable: variable.mutable,
            }),
            Symbol::Function(function) => ResolvedSymbol::Function(function.clone()),
            Symbol::BuiltinFunction(builtin) => ResolvedSymbol::BuiltinFunction(*builtin),
        };

        self.semantics.references.insert(span, resolved);
    }
}

fn line_column(source: &str, index: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;

    for (offset, ch) in source.char_indices() {
        if offset >= index {
            break;
        }

        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    (line, column)
}

fn is_numeric(type_name: TypeName) -> bool {
    matches!(type_name, TypeName::Int | TypeName::Double)
}

fn promote_numeric(left: TypeName, right: TypeName) -> TypeName {
    if left == TypeName::Double || right == TypeName::Double {
        TypeName::Double
    } else {
        TypeName::Int
    }
}

fn is_assignable(actual: TypeName, expected: TypeName) -> bool {
    actual == expected || (actual == TypeName::Int && expected == TypeName::Double)
}

fn types_compatible(left: TypeName, right: TypeName) -> bool {
    left == right || (is_numeric(left) && is_numeric(right))
}

fn describe_type(type_name: TypeName) -> &'static str {
    match type_name {
        TypeName::Int => "int",
        TypeName::Double => "double",
        TypeName::Bool => "bool",
        TypeName::Void => "void",
    }
}

fn contains_return_statement(block: &BlockStatement) -> bool {
    block.statements.iter().any(statement_contains_return)
}

fn statement_contains_return(statement: &Statement) -> bool {
    match statement {
        Statement::Return(_) => true,
        Statement::Block(block) => contains_return_statement(block),
        Statement::While(while_statement) => contains_return_statement(&while_statement.body),
        Statement::FunctionDeclaration(_) => false,
        Statement::VariableDeclaration(_) | Statement::Assignment(_) | Statement::Expression(_) => {
            false
        }
    }
}
