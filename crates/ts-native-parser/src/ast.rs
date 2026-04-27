use crate::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub has_module_preamble: bool,
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    VariableDeclaration(VariableDeclaration),
    Assignment(AssignmentStatement),
    Expression(ExpressionStatement),
    FunctionDeclaration(FunctionDeclaration),
    Return(ReturnStatement),
    While(WhileStatement),
    Block(BlockStatement),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDeclaration {
    pub name: Identifier,
    pub parameters: Vec<FunctionParameter>,
    pub return_type: TypeName,
    pub body: BlockStatement,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionParameter {
    pub name: Identifier,
    pub type_annotation: TypeName,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    Const,
    Let,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariableDeclaration {
    pub kind: BindingKind,
    pub name: Identifier,
    pub type_annotation: Option<TypeName>,
    pub initializer: Expression,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssignmentStatement {
    pub target: Identifier,
    pub value: Expression,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpressionStatement {
    pub expression: Expression,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStatement {
    pub value: Option<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhileStatement {
    pub condition: Expression,
    pub body: BlockStatement,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockStatement {
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    IntegerLiteral(IntegerLiteral),
    DoubleLiteral(DoubleLiteral),
    BoolLiteral(BoolLiteral),
    Identifier(Identifier),
    Unary(UnaryExpression),
    Binary(BinaryExpression),
    Call(CallExpression),
    ConsoleLog(ConsoleLogExpression),
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntegerLiteral {
    pub value: i64,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DoubleLiteral {
    pub value: f64,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoolLiteral {
    pub value: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Negate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnaryExpression {
    pub operator: UnaryOperator,
    pub operand: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
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

#[derive(Debug, Clone, PartialEq)]
pub struct BinaryExpression {
    pub left: Box<Expression>,
    pub operator: BinaryOperator,
    pub right: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallExpression {
    pub callee: Box<Expression>,
    pub arguments: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConsoleLogExpression {
    pub arguments: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeName {
    Int,
    Double,
    Bool,
    Void,
}

impl Expression {
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::IntegerLiteral(literal) => literal.span,
            Self::DoubleLiteral(literal) => literal.span,
            Self::BoolLiteral(literal) => literal.span,
            Self::Identifier(identifier) => identifier.span,
            Self::Unary(expression) => expression.span,
            Self::Binary(expression) => expression.span,
            Self::Call(expression) => expression.span,
            Self::ConsoleLog(expression) => expression.span,
        }
    }

    #[must_use]
    pub fn with_span(self, span: Span) -> Self {
        match self {
            Self::IntegerLiteral(mut literal) => {
                literal.span = span;
                Self::IntegerLiteral(literal)
            }
            Self::DoubleLiteral(mut literal) => {
                literal.span = span;
                Self::DoubleLiteral(literal)
            }
            Self::BoolLiteral(mut literal) => {
                literal.span = span;
                Self::BoolLiteral(literal)
            }
            Self::Identifier(mut identifier) => {
                identifier.span = span;
                Self::Identifier(identifier)
            }
            Self::Unary(mut expression) => {
                expression.span = span;
                Self::Unary(expression)
            }
            Self::Binary(mut expression) => {
                expression.span = span;
                Self::Binary(expression)
            }
            Self::Call(mut expression) => {
                expression.span = span;
                Self::Call(expression)
            }
            Self::ConsoleLog(mut expression) => {
                expression.span = span;
                Self::ConsoleLog(expression)
            }
        }
    }
}
