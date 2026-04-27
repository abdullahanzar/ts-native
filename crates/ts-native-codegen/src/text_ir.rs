use std::collections::HashMap;

use ts_native_ir::{
    IrAssignment, IrBinaryOperator, IrBlock, IrExpression, IrExpressionKind, IrFunction, IrModule,
    IrParameter, IrStatement, IrType, IrUnaryOperator, IrVariableDeclaration, IrWhileStatement,
};

use crate::CodegenError;

const BUILTIN_PRINT_INT: &str = "printInt";
const BUILTIN_PRINT_DOUBLE: &str = "printDouble";
const BUILTIN_PRINT_BOOL: &str = "printBool";
const RUNTIME_PRINT_INT: &str = "__tsn_print_int";
const RUNTIME_PRINT_DOUBLE: &str = "__tsn_print_double";
const RUNTIME_PRINT_BOOL: &str = "__tsn_print_bool";

pub(crate) fn emit_llvm_ir(module: &IrModule) -> Result<String, CodegenError> {
    TextLlvmEmitter::new().emit_module(module)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FunctionSignature {
    return_type: IrType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StackSlot {
    pointer_name: String,
    type_name: IrType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EmittedValue {
    operand: Option<String>,
    type_name: IrType,
}

struct TextLlvmEmitter {
    lines: Vec<String>,
    temp_index: usize,
    label_index: usize,
    function_signatures: HashMap<String, FunctionSignature>,
    scopes: Vec<HashMap<String, StackSlot>>,
    block_terminated: bool,
    current_return_type: Option<IrType>,
}

impl TextLlvmEmitter {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            temp_index: 0,
            label_index: 0,
            function_signatures: HashMap::new(),
            scopes: Vec::new(),
            block_terminated: false,
            current_return_type: None,
        }
    }

    fn emit_module(mut self, module: &IrModule) -> Result<String, CodegenError> {
        self.collect_function_signatures(&module.statements)?;
        self.declare_builtin_functions()?;
        self.emit_runtime_declarations();
        self.emit_builtin_wrappers();

        let mut top_level_statements = Vec::new();

        for statement in &module.statements {
            match statement {
                IrStatement::FunctionDeclaration(function) => self.emit_function(function)?,
                _ => top_level_statements.push(statement),
            }
        }

        if !top_level_statements.is_empty() {
            self.emit_top_level_entry(&top_level_statements)?;
        }

        Ok(self.lines.join("\n"))
    }

    fn declare_builtin_functions(&mut self) -> Result<(), CodegenError> {
        for name in [BUILTIN_PRINT_INT, BUILTIN_PRINT_DOUBLE, BUILTIN_PRINT_BOOL] {
            let duplicate = self.function_signatures.insert(
                name.to_owned(),
                FunctionSignature {
                    return_type: IrType::Void,
                },
            );

            if duplicate.is_some() {
                return Err(CodegenError::message(format!(
                    "duplicate function declaration `{name}` in TS-Native IR"
                )));
            }
        }

        Ok(())
    }

    fn emit_runtime_declarations(&mut self) {
        self.lines
            .push(format!("declare void @{RUNTIME_PRINT_INT}(i64)"));
        self.lines
            .push(format!("declare void @{RUNTIME_PRINT_DOUBLE}(double)"));
        self.lines
            .push(format!("declare void @{RUNTIME_PRINT_BOOL}(i64)"));
    }

    fn emit_builtin_wrappers(&mut self) {
        self.emit_builtin_wrapper(BUILTIN_PRINT_INT, IrType::Int, RUNTIME_PRINT_INT, None);
        self.emit_builtin_wrapper(
            BUILTIN_PRINT_DOUBLE,
            IrType::Double,
            RUNTIME_PRINT_DOUBLE,
            None,
        );
        self.emit_builtin_wrapper(
            BUILTIN_PRINT_BOOL,
            IrType::Bool,
            RUNTIME_PRINT_BOOL,
            Some(IrType::Int),
        );
    }

    fn emit_builtin_wrapper(
        &mut self,
        name: &str,
        parameter_type: IrType,
        runtime_name: &str,
        widened_argument_type: Option<IrType>,
    ) {
        self.begin_function(
            name,
            &[IrParameter {
                name: "value".to_owned(),
                type_name: parameter_type,
            }],
            IrType::Void,
            Some("entry"),
        );

        let argument = match widened_argument_type {
            Some(target_type) => {
                let widened = self.next_temp("builtin.arg");
                self.push_instruction(&format!(
                    "{widened} = zext {} %value to {}",
                    llvm_type(parameter_type),
                    llvm_type(target_type)
                ));
                format!("{} {widened}", llvm_type(target_type))
            }
            None => format!("{} %value", llvm_type(parameter_type)),
        };

        self.push_instruction(&format!("call void @{runtime_name}({argument})"));
        self.push_instruction("ret void");
        self.end_function();
    }

    fn collect_function_signatures(
        &mut self,
        statements: &[IrStatement],
    ) -> Result<(), CodegenError> {
        for statement in statements {
            if let IrStatement::FunctionDeclaration(function) = statement {
                let duplicate = self.function_signatures.insert(
                    function.name.clone(),
                    FunctionSignature {
                        return_type: function.return_type,
                    },
                );

                if duplicate.is_some() {
                    return Err(CodegenError::message(format!(
                        "duplicate function declaration `{}` in TS-Native IR",
                        function.name
                    )));
                }
            }
        }

        Ok(())
    }

    fn emit_function(&mut self, function: &IrFunction) -> Result<(), CodegenError> {
        self.begin_function(
            &function.name,
            &function.parameters,
            function.return_type,
            Some("entry"),
        );

        self.push_scope();
        for parameter in &function.parameters {
            self.bind_parameter(parameter)?;
        }

        let previous_return_type = self.current_return_type;
        self.current_return_type = Some(function.return_type);
        self.emit_statements(&function.body.statements)?;
        self.current_return_type = previous_return_type;

        if !self.block_terminated {
            if function.return_type == IrType::Void {
                self.push_instruction("ret void");
                self.block_terminated = true;
            } else {
                return Err(CodegenError::message(format!(
                    "function `{}` does not end in an explicit return",
                    function.name
                )));
            }
        }

        self.pop_scope()?;
        self.end_function();
        Ok(())
    }

    fn emit_top_level_entry(&mut self, statements: &[&IrStatement]) -> Result<(), CodegenError> {
        self.begin_function("__tsn_entry", &[], IrType::Void, Some("entry"));
        self.push_scope();

        let previous_return_type = self.current_return_type;
        self.current_return_type = Some(IrType::Void);
        for statement in statements {
            self.emit_statement(statement)?;
        }
        self.current_return_type = previous_return_type;

        if !self.block_terminated {
            self.push_instruction("ret void");
            self.block_terminated = true;
        }

        self.pop_scope()?;
        self.end_function();
        Ok(())
    }

    fn begin_function(
        &mut self,
        name: &str,
        parameters: &[IrParameter],
        return_type: IrType,
        entry_label: Option<&str>,
    ) {
        if !self.lines.is_empty() {
            self.lines.push(String::new());
        }

        let parameters = parameters
            .iter()
            .map(|parameter| format!("{} %{}", llvm_type(parameter.type_name), parameter.name))
            .collect::<Vec<_>>()
            .join(", ");

        self.lines.push(format!(
            "define {} @{}({}) {{",
            llvm_type(return_type),
            name,
            parameters
        ));

        if let Some(entry_label) = entry_label {
            self.begin_block(entry_label);
        }
    }

    fn end_function(&mut self) {
        self.lines.push("}".to_string());
        self.block_terminated = false;
    }

    fn bind_parameter(&mut self, parameter: &IrParameter) -> Result<(), CodegenError> {
        let slot = self.next_named_value(&parameter.name, "addr");
        let type_name = llvm_type(parameter.type_name);

        self.push_instruction(&format!("{slot} = alloca {type_name}"));
        self.push_instruction(&format!(
            "store {type_name} %{}, {} {slot}",
            parameter.name,
            llvm_pointer_type(parameter.type_name)
        ));

        self.bind_variable(
            parameter.name.clone(),
            StackSlot {
                pointer_name: slot,
                type_name: parameter.type_name,
            },
        )
    }

    fn emit_statements(&mut self, statements: &[IrStatement]) -> Result<(), CodegenError> {
        for statement in statements {
            self.emit_statement(statement)?;
        }

        Ok(())
    }

    fn emit_statement(&mut self, statement: &IrStatement) -> Result<(), CodegenError> {
        if self.block_terminated {
            return Err(CodegenError::message(
                "encountered an unreachable statement after a terminator in TS-Native IR",
            ));
        }

        match statement {
            IrStatement::VariableDeclaration(declaration) => {
                self.emit_variable_declaration(declaration)
            }
            IrStatement::Assignment(assignment) => self.emit_assignment(assignment),
            IrStatement::Expression(expression) => {
                self.emit_expression(expression)?;
                Ok(())
            }
            IrStatement::FunctionDeclaration(function) => Err(CodegenError::message(format!(
                "nested function declaration `{}` is not supported during LLVM IR emission",
                function.name
            ))),
            IrStatement::Return(value) => self.emit_return(value.as_ref()),
            IrStatement::While(while_statement) => self.emit_while(while_statement),
            IrStatement::Block(block) => self.emit_block(block),
        }
    }

    fn emit_variable_declaration(
        &mut self,
        declaration: &IrVariableDeclaration,
    ) -> Result<(), CodegenError> {
        let initializer = self.emit_expression(&declaration.initializer)?;
        let value = self.expect_value(initializer, "variable initializer")?;
        let slot = self.next_named_value(&declaration.name, "addr");
        let type_name = llvm_type(declaration.type_name);

        self.push_instruction(&format!("{slot} = alloca {type_name}"));
        self.push_instruction(&format!(
            "store {type_name} {value}, {} {slot}",
            llvm_pointer_type(declaration.type_name)
        ));

        self.bind_variable(
            declaration.name.clone(),
            StackSlot {
                pointer_name: slot,
                type_name: declaration.type_name,
            },
        )
    }

    fn emit_assignment(&mut self, assignment: &IrAssignment) -> Result<(), CodegenError> {
        let slot = self.lookup_variable(&assignment.target)?.clone();
        if slot.type_name != assignment.type_name {
            return Err(CodegenError::message(format!(
                "assignment to `{}` changed type from {} to {}",
                assignment.target, slot.type_name, assignment.type_name
            )));
        }

        let value_expression = self.emit_expression(&assignment.value)?;
        let value = self.expect_value(value_expression, "assignment value")?;
        let type_name = llvm_type(assignment.type_name);
        self.push_instruction(&format!(
            "store {type_name} {value}, {} {}",
            llvm_pointer_type(assignment.type_name),
            slot.pointer_name
        ));
        Ok(())
    }

    fn emit_return(&mut self, value: Option<&IrExpression>) -> Result<(), CodegenError> {
        let expected_return_type = self.current_return_type.ok_or_else(|| {
            CodegenError::message("return statement encountered outside of a function context")
        })?;

        match value {
            Some(expression) => {
                let emitted = self.emit_expression(expression)?;
                let operand = self.expect_value(emitted, "return value")?;
                self.push_instruction(&format!(
                    "ret {} {operand}",
                    llvm_type(expected_return_type)
                ));
            }
            None => {
                if expected_return_type != IrType::Void {
                    return Err(CodegenError::message(format!(
                        "non-void function attempted to return without a value ({expected_return_type})"
                    )));
                }
                self.push_instruction("ret void");
            }
        }

        self.block_terminated = true;
        Ok(())
    }

    fn emit_while(&mut self, while_statement: &IrWhileStatement) -> Result<(), CodegenError> {
        let condition_label = self.next_label("while.cond");
        let body_label = self.next_label("while.body");
        let end_label = self.next_label("while.end");

        self.push_instruction(&format!("br label %{condition_label}"));
        self.block_terminated = true;

        self.begin_block(&condition_label);
        let condition = self.emit_expression(&while_statement.condition)?;
        let condition_operand = self.expect_value(condition, "while condition")?;
        self.push_instruction(&format!(
            "br i1 {condition_operand}, label %{body_label}, label %{end_label}"
        ));
        self.block_terminated = true;

        self.begin_block(&body_label);
        self.push_scope();
        self.emit_statements(&while_statement.body.statements)?;
        self.pop_scope()?;
        if !self.block_terminated {
            self.push_instruction(&format!("br label %{condition_label}"));
            self.block_terminated = true;
        }

        self.begin_block(&end_label);
        Ok(())
    }

    fn emit_block(&mut self, block: &IrBlock) -> Result<(), CodegenError> {
        self.push_scope();
        self.emit_statements(&block.statements)?;
        self.pop_scope()?;
        Ok(())
    }

    fn emit_expression(&mut self, expression: &IrExpression) -> Result<EmittedValue, CodegenError> {
        match &expression.kind {
            IrExpressionKind::IntegerLiteral(value) => Ok(EmittedValue {
                operand: Some(value.to_string()),
                type_name: expression.type_name,
            }),
            IrExpressionKind::DoubleLiteral(value) => Ok(EmittedValue {
                operand: Some(value.clone()),
                type_name: expression.type_name,
            }),
            IrExpressionKind::BoolLiteral(value) => Ok(EmittedValue {
                operand: Some(if *value { "1" } else { "0" }.to_string()),
                type_name: expression.type_name,
            }),
            IrExpressionKind::Variable(name) => self.emit_variable_load(name, expression.type_name),
            IrExpressionKind::Unary { operator, operand } => {
                self.emit_unary_expression(*operator, operand, expression.type_name)
            }
            IrExpressionKind::Binary {
                left,
                operator,
                right,
            } => self.emit_binary_expression(left, *operator, right, expression.type_name),
            IrExpressionKind::Call { callee, arguments } => {
                self.emit_call_expression(callee, arguments, expression.type_name)
            }
            IrExpressionKind::Cast {
                expression,
                target_type,
            } => self.emit_cast_expression(expression, *target_type),
        }
    }

    fn emit_variable_load(
        &mut self,
        name: &str,
        type_name: IrType,
    ) -> Result<EmittedValue, CodegenError> {
        let slot = self.lookup_variable(name)?.clone();
        if slot.type_name != type_name {
            return Err(CodegenError::message(format!(
                "variable `{name}` resolved to {} instead of expected {type_name}",
                slot.type_name
            )));
        }

        let load_name = self.next_temp("load");
        self.push_instruction(&format!(
            "{load_name} = load {}, {} {}",
            llvm_type(type_name),
            llvm_pointer_type(type_name),
            slot.pointer_name
        ));

        Ok(EmittedValue {
            operand: Some(load_name),
            type_name,
        })
    }

    fn emit_unary_expression(
        &mut self,
        operator: IrUnaryOperator,
        operand: &IrExpression,
        result_type: IrType,
    ) -> Result<EmittedValue, CodegenError> {
        let emitted_operand = self.emit_expression(operand)?;
        let operand = self.expect_value(emitted_operand, "unary operand")?;
        let name = self.next_temp("neg");

        match (operator, result_type) {
            (IrUnaryOperator::Negate, IrType::Int) => {
                self.push_instruction(&format!("{name} = sub i64 0, {operand}"));
            }
            (IrUnaryOperator::Negate, IrType::Double) => {
                self.push_instruction(&format!("{name} = fsub double -0.0, {operand}"));
            }
            _ => {
                return Err(CodegenError::message(format!(
                    "unsupported unary operator/type combination during LLVM emission: {operator:?} {result_type}"
                )));
            }
        }

        Ok(EmittedValue {
            operand: Some(name),
            type_name: result_type,
        })
    }

    fn emit_binary_expression(
        &mut self,
        left: &IrExpression,
        operator: IrBinaryOperator,
        right: &IrExpression,
        result_type: IrType,
    ) -> Result<EmittedValue, CodegenError> {
        let left_value = self.emit_expression(left)?;
        let right_value = self.emit_expression(right)?;
        let left_operand = self.expect_value(left_value.clone(), "binary left operand")?;
        let right_operand = self.expect_value(right_value.clone(), "binary right operand")?;
        let instruction_name = self.next_temp("tmp");

        let instruction = match operator {
            IrBinaryOperator::Add => arithmetic_instruction("add", "fadd", left_value.type_name),
            IrBinaryOperator::Subtract => {
                arithmetic_instruction("sub", "fsub", left_value.type_name)
            }
            IrBinaryOperator::Multiply => {
                arithmetic_instruction("mul", "fmul", left_value.type_name)
            }
            IrBinaryOperator::Divide => {
                arithmetic_instruction("sdiv", "fdiv", left_value.type_name)
            }
            IrBinaryOperator::Less => {
                comparison_instruction("icmp slt", "fcmp olt", left_value.type_name)
            }
            IrBinaryOperator::LessEqual => {
                comparison_instruction("icmp sle", "fcmp ole", left_value.type_name)
            }
            IrBinaryOperator::Greater => {
                comparison_instruction("icmp sgt", "fcmp ogt", left_value.type_name)
            }
            IrBinaryOperator::GreaterEqual => {
                comparison_instruction("icmp sge", "fcmp oge", left_value.type_name)
            }
            IrBinaryOperator::Equal => {
                comparison_instruction("icmp eq", "fcmp oeq", left_value.type_name)
            }
            IrBinaryOperator::NotEqual => {
                comparison_instruction("icmp ne", "fcmp one", left_value.type_name)
            }
        }?;

        self.push_instruction(&format!(
            "{instruction_name} = {instruction} {} {left_operand}, {right_operand}",
            llvm_type(left_value.type_name)
        ));

        Ok(EmittedValue {
            operand: Some(instruction_name),
            type_name: result_type,
        })
    }

    fn emit_call_expression(
        &mut self,
        callee: &str,
        arguments: &[IrExpression],
        return_type: IrType,
    ) -> Result<EmittedValue, CodegenError> {
        let signature = self
            .function_signatures
            .get(callee)
            .copied()
            .ok_or_else(|| {
                CodegenError::message(format!("unknown function `{callee}` in TS-Native IR"))
            })?;

        let mut emitted_arguments = Vec::with_capacity(arguments.len());
        for argument in arguments {
            let emitted = self.emit_expression(argument)?;
            let operand = self.expect_value(emitted, "call argument")?;
            emitted_arguments.push(format!("{} {operand}", llvm_type(argument.type_name)));
        }
        let arguments = emitted_arguments.join(", ");

        if signature.return_type == IrType::Void {
            self.push_instruction(&format!("call void @{callee}({arguments})"));
            return Ok(EmittedValue {
                operand: None,
                type_name: IrType::Void,
            });
        }

        let call_name = self.next_temp("call");
        self.push_instruction(&format!(
            "{call_name} = call {} @{callee}({arguments})",
            llvm_type(signature.return_type)
        ));

        Ok(EmittedValue {
            operand: Some(call_name),
            type_name: return_type,
        })
    }

    fn emit_cast_expression(
        &mut self,
        expression: &IrExpression,
        target_type: IrType,
    ) -> Result<EmittedValue, CodegenError> {
        let value = self.emit_expression(expression)?;
        let operand = self.expect_value(value.clone(), "cast operand")?;
        let cast_name = self.next_temp("cast");

        match (value.type_name, target_type) {
            (IrType::Int, IrType::Double) => {
                self.push_instruction(&format!("{cast_name} = sitofp i64 {operand} to double"));
            }
            _ => {
                return Err(CodegenError::message(format!(
                    "unsupported cast during LLVM emission: {} to {}",
                    value.type_name, target_type
                )));
            }
        }

        Ok(EmittedValue {
            operand: Some(cast_name),
            type_name: target_type,
        })
    }

    fn expect_value(&self, value: EmittedValue, context: &str) -> Result<String, CodegenError> {
        value.operand.ok_or_else(|| {
            CodegenError::message(format!(
                "expected a value-producing expression while emitting {context}, but got `{}`",
                value.type_name
            ))
        })
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) -> Result<(), CodegenError> {
        self.scopes.pop().map(|_| ()).ok_or_else(|| {
            CodegenError::message("attempted to pop an empty code generation scope stack")
        })
    }

    fn bind_variable(&mut self, name: String, slot: StackSlot) -> Result<(), CodegenError> {
        let scope = self.scopes.last_mut().ok_or_else(|| {
            CodegenError::message("attempted to bind a variable without an active scope")
        })?;

        scope.insert(name, slot);
        Ok(())
    }

    fn lookup_variable(&self, name: &str) -> Result<&StackSlot, CodegenError> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .ok_or_else(|| {
                CodegenError::message(format!("unknown variable `{name}` in TS-Native IR"))
            })
    }

    fn begin_block(&mut self, label: &str) {
        self.lines.push(format!("{label}:"));
        self.block_terminated = false;
    }

    fn push_instruction(&mut self, instruction: &str) {
        self.lines.push(format!("  {instruction}"));
    }

    fn next_temp(&mut self, prefix: &str) -> String {
        let name = format!("%{prefix}.{}", self.temp_index);
        self.temp_index += 1;
        name
    }

    fn next_named_value(&mut self, name: &str, suffix: &str) -> String {
        let value = format!("%{name}.{suffix}.{}", self.temp_index);
        self.temp_index += 1;
        value
    }

    fn next_label(&mut self, prefix: &str) -> String {
        let label = format!("{prefix}.{}", self.label_index);
        self.label_index += 1;
        label
    }
}

fn llvm_type(type_name: IrType) -> &'static str {
    match type_name {
        IrType::Int => "i64",
        IrType::Double => "double",
        IrType::Bool => "i1",
        IrType::Void => "void",
    }
}

fn llvm_pointer_type(type_name: IrType) -> String {
    format!("{}*", llvm_type(type_name))
}

fn arithmetic_instruction(
    integer_instruction: &'static str,
    float_instruction: &'static str,
    operand_type: IrType,
) -> Result<&'static str, CodegenError> {
    match operand_type {
        IrType::Int => Ok(integer_instruction),
        IrType::Double => Ok(float_instruction),
        _ => Err(CodegenError::message(format!(
            "arithmetic instruction requested for non-numeric LLVM operand type {operand_type}"
        ))),
    }
}

fn comparison_instruction(
    integer_instruction: &'static str,
    float_instruction: &'static str,
    operand_type: IrType,
) -> Result<&'static str, CodegenError> {
    match operand_type {
        IrType::Int | IrType::Bool => Ok(integer_instruction),
        IrType::Double => Ok(float_instruction),
        IrType::Void => Err(CodegenError::message(
            "comparison instruction requested for void LLVM operand type",
        )),
    }
}
