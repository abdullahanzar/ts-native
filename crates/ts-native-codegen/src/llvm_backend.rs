use std::{collections::HashMap, path::Path};

use inkwell::{
    FloatPredicate, IntPredicate, OptimizationLevel,
    builder::{Builder, BuilderError},
    context::Context,
    module::Module,
    targets::{CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine},
    types::{BasicMetadataTypeEnum, BasicTypeEnum, FunctionType},
    values::{BasicValueEnum, FloatValue, FunctionValue, IntValue, PointerValue},
};
use ts_native_ir::{
    IrAssignment, IrBinaryOperator, IrBlock, IrExpression, IrExpressionKind, IrFunction, IrModule,
    IrParameter, IrStatement, IrType, IrUnaryOperator, IrVariableDeclaration, IrWhileStatement,
};

use crate::CodegenError;

const GENERATED_ENTRY_FUNCTION: &str = "__tsn_entry";
const GENERATED_MAIN_FUNCTION: &str = "main";
const BUILTIN_PRINT_INT: &str = "printInt";
const BUILTIN_PRINT_DOUBLE: &str = "printDouble";
const BUILTIN_PRINT_BOOL: &str = "printBool";
const RUNTIME_PRINT_INT: &str = "__tsn_print_int";
const RUNTIME_PRINT_DOUBLE: &str = "__tsn_print_double";
const RUNTIME_PRINT_BOOL: &str = "__tsn_print_bool";

pub(crate) fn emit_object_file(module: &IrModule, output_path: &Path) -> Result<(), CodegenError> {
    Target::initialize_native(&InitializationConfig::default()).map_err(|error| {
        CodegenError::message(format!(
            "failed to initialize the native LLVM target: {error}"
        ))
    })?;

    let triple = TargetMachine::get_default_triple();
    let target = Target::from_triple(&triple).map_err(|error| {
        CodegenError::message(format!(
            "failed to resolve LLVM target from host triple: {error}"
        ))
    })?;

    let cpu = TargetMachine::get_host_cpu_name().to_string();
    let features = TargetMachine::get_host_cpu_features().to_string();
    let target_machine = target
        .create_target_machine(
            &triple,
            &cpu,
            &features,
            OptimizationLevel::None,
            RelocMode::Default,
            CodeModel::Default,
        )
        .ok_or_else(|| {
            CodegenError::message("failed to create an LLVM target machine for the host")
        })?;

    let context = Context::create();
    let llvm_module = context.create_module("ts_native");
    llvm_module.set_triple(&triple);
    let data_layout = target_machine.get_target_data().get_data_layout();
    llvm_module.set_data_layout(&data_layout);

    let mut backend = NativeLlvmBackend::new(&context, llvm_module);
    backend.emit_module(module)?;
    backend.module.verify().map_err(|error| {
        CodegenError::message(format!(
            "generated LLVM module failed verification: {error}\n{}",
            backend.module.to_string()
        ))
    })?;

    target_machine
        .write_to_file(&backend.module, FileType::Object, output_path)
        .map_err(|error| {
            CodegenError::message(format!(
                "failed to write native object file to {}: {error}",
                output_path.display()
            ))
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FunctionSignature {
    parameters: Vec<IrType>,
    return_type: IrType,
}

#[derive(Debug, Clone, Copy)]
struct StackSlot<'ctx> {
    pointer: PointerValue<'ctx>,
    type_name: IrType,
}

#[derive(Debug, Clone, Copy)]
struct LlvmValue<'ctx> {
    value: Option<BasicValueEnum<'ctx>>,
    type_name: IrType,
}

struct NativeLlvmBackend<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    function_signatures: HashMap<String, FunctionSignature>,
    function_values: HashMap<String, FunctionValue<'ctx>>,
    scopes: Vec<HashMap<String, StackSlot<'ctx>>>,
    current_function: Option<FunctionValue<'ctx>>,
    current_return_type: Option<IrType>,
    block_terminated: bool,
    name_counter: usize,
    label_counter: usize,
}

impl<'ctx> NativeLlvmBackend<'ctx> {
    fn new(context: &'ctx Context, module: Module<'ctx>) -> Self {
        Self {
            context,
            module,
            builder: context.create_builder(),
            function_signatures: HashMap::new(),
            function_values: HashMap::new(),
            scopes: Vec::new(),
            current_function: None,
            current_return_type: None,
            block_terminated: false,
            name_counter: 0,
            label_counter: 0,
        }
    }

    fn emit_module(&mut self, ir_module: &IrModule) -> Result<(), CodegenError> {
        self.collect_function_signatures(&ir_module.statements)?;
        self.declare_builtin_functions()?;
        self.declare_user_functions()?;
        self.emit_builtin_wrappers()?;

        let mut top_level_statements = Vec::new();
        for statement in &ir_module.statements {
            match statement {
                IrStatement::FunctionDeclaration(function) => self.emit_function_body(function)?,
                _ => top_level_statements.push(statement),
            }
        }

        if !top_level_statements.is_empty() {
            self.emit_top_level_entry(&top_level_statements)?;
        }

        self.emit_host_main(!top_level_statements.is_empty())
    }

    fn declare_builtin_functions(&mut self) -> Result<(), CodegenError> {
        for (name, parameters) in [
            (BUILTIN_PRINT_INT, vec![IrType::Int]),
            (BUILTIN_PRINT_DOUBLE, vec![IrType::Double]),
            (BUILTIN_PRINT_BOOL, vec![IrType::Bool]),
        ] {
            let duplicate = self.function_signatures.insert(
                name.to_owned(),
                FunctionSignature {
                    parameters,
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

    fn collect_function_signatures(
        &mut self,
        statements: &[IrStatement],
    ) -> Result<(), CodegenError> {
        for statement in statements {
            if let IrStatement::FunctionDeclaration(function) = statement {
                if matches!(
                    function.name.as_str(),
                    GENERATED_ENTRY_FUNCTION | GENERATED_MAIN_FUNCTION
                ) {
                    return Err(CodegenError::message(format!(
                        "native emission reserves the function name `{}`",
                        function.name
                    )));
                }

                let duplicate = self.function_signatures.insert(
                    function.name.clone(),
                    FunctionSignature {
                        parameters: function
                            .parameters
                            .iter()
                            .map(|parameter| parameter.type_name)
                            .collect(),
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

    fn declare_user_functions(&mut self) -> Result<(), CodegenError> {
        for (name, signature) in &self.function_signatures {
            let function_type = self.function_type(&signature.parameters, signature.return_type)?;
            let function_value = self.module.add_function(name, function_type, None);
            self.function_values.insert(name.clone(), function_value);
        }

        Ok(())
    }

    fn emit_builtin_wrappers(&mut self) -> Result<(), CodegenError> {
        self.emit_builtin_wrapper(BUILTIN_PRINT_INT, RUNTIME_PRINT_INT, None)?;
        self.emit_builtin_wrapper(BUILTIN_PRINT_DOUBLE, RUNTIME_PRINT_DOUBLE, None)?;
        self.emit_builtin_wrapper(BUILTIN_PRINT_BOOL, RUNTIME_PRINT_BOOL, Some(IrType::Int))?;
        Ok(())
    }

    fn emit_builtin_wrapper(
        &mut self,
        builtin_name: &str,
        runtime_name: &str,
        widened_argument_type: Option<IrType>,
    ) -> Result<(), CodegenError> {
        let wrapper_function = self.lookup_function(builtin_name)?;
        let runtime_function = self.declare_runtime_function(runtime_name, widened_argument_type)?;
        let entry_block = self.context.append_basic_block(wrapper_function, "entry");
        self.builder.position_at_end(entry_block);

        let argument = wrapper_function.get_first_param().ok_or_else(|| {
            CodegenError::message(format!(
                "builtin wrapper `{builtin_name}` is missing its argument"
            ))
        })?;

        let argument = match widened_argument_type {
            Some(target_type) => {
                let argument = argument.into_int_value();
                let zext_name = self.next_name("builtin.zext");
                let widened = map_builder(
                    self.builder.build_int_z_extend(
                        argument,
                        self.integer_type(target_type)?,
                        &zext_name,
                    ),
                    "zero-extend builtin print bool argument",
                )?;
                widened.into()
            }
            None => argument,
        };

        let call_name = self.next_name("builtin.call");
        map_builder(
            self.builder
                .build_call(runtime_function, &[argument.into()], &call_name),
            "emit builtin runtime call",
        )?;
        map_builder(
            self.builder.build_return(None),
            "emit builtin wrapper return",
        )?;
        Ok(())
    }

    fn declare_runtime_function(
        &mut self,
        name: &str,
        widened_argument_type: Option<IrType>,
    ) -> Result<FunctionValue<'ctx>, CodegenError> {
        if let Some(function) = self.module.get_function(name) {
            return Ok(function);
        }

        let parameter_type = match widened_argument_type.unwrap_or_else(|| builtin_runtime_argument_type(name)) {
            IrType::Int => self.context.i64_type().into(),
            IrType::Double => self.context.f64_type().into(),
            IrType::Bool | IrType::Void => {
                return Err(CodegenError::message(format!(
                    "unsupported runtime parameter type for `{name}`"
                )))
            }
        };

        Ok(self.module.add_function(
            name,
            self.context.void_type().fn_type(&[parameter_type], false),
            None,
        ))
    }

    fn emit_function_body(&mut self, function: &IrFunction) -> Result<(), CodegenError> {
        let function_value = self.lookup_function(function.name.as_str())?;
        let entry_block = self.context.append_basic_block(function_value, "entry");
        self.builder.position_at_end(entry_block);

        let previous_function = self.current_function;
        let previous_return_type = self.current_return_type;
        let previous_block_terminated = self.block_terminated;

        self.current_function = Some(function_value);
        self.current_return_type = Some(function.return_type);
        self.block_terminated = false;
        self.push_scope();

        for (index, parameter) in function.parameters.iter().enumerate() {
            self.bind_parameter(function_value, index, parameter)?;
        }

        self.emit_statements(&function.body.statements)?;

        if !self.block_terminated {
            if function.return_type == IrType::Void {
                map_builder(
                    self.builder.build_return(None),
                    "emit implicit void return for function body",
                )?;
                self.block_terminated = true;
            } else {
                return Err(CodegenError::message(format!(
                    "function `{}` does not end in an explicit return",
                    function.name
                )));
            }
        }

        self.pop_scope()?;
        self.current_function = previous_function;
        self.current_return_type = previous_return_type;
        self.block_terminated = previous_block_terminated;
        Ok(())
    }

    fn emit_top_level_entry(&mut self, statements: &[&IrStatement]) -> Result<(), CodegenError> {
        let function_value = self.module.add_function(
            GENERATED_ENTRY_FUNCTION,
            self.context.void_type().fn_type(&[], false),
            None,
        );
        let entry_block = self.context.append_basic_block(function_value, "entry");
        self.builder.position_at_end(entry_block);

        let previous_function = self.current_function;
        let previous_return_type = self.current_return_type;
        let previous_block_terminated = self.block_terminated;

        self.current_function = Some(function_value);
        self.current_return_type = Some(IrType::Void);
        self.block_terminated = false;
        self.push_scope();

        for statement in statements {
            self.emit_statement(statement)?;
        }

        if !self.block_terminated {
            map_builder(
                self.builder.build_return(None),
                "emit generated return for top-level entry function",
            )?;
            self.block_terminated = true;
        }

        self.pop_scope()?;
        self.current_function = previous_function;
        self.current_return_type = previous_return_type;
        self.block_terminated = previous_block_terminated;
        Ok(())
    }

    fn emit_host_main(&mut self, has_top_level_entry: bool) -> Result<(), CodegenError> {
        let main_function = self.module.add_function(
            GENERATED_MAIN_FUNCTION,
            self.context.i32_type().fn_type(&[], false),
            None,
        );
        let entry_block = self.context.append_basic_block(main_function, "entry");
        self.builder.position_at_end(entry_block);

        if has_top_level_entry {
            let entry_function = self
                .module
                .get_function(GENERATED_ENTRY_FUNCTION)
                .ok_or_else(|| {
                    CodegenError::message(
                        "missing generated top-level entry function during native emission",
                    )
                })?;

            map_builder(
                self.builder
                    .build_call(entry_function, &[], "call_tsn_entry"),
                "call generated top-level entry function from host main",
            )?;
        }

        map_builder(
            self.builder
                .build_return(Some(&self.context.i32_type().const_zero())),
            "emit host main return",
        )?;
        Ok(())
    }

    fn bind_parameter(
        &mut self,
        function_value: FunctionValue<'ctx>,
        index: usize,
        parameter: &IrParameter,
    ) -> Result<(), CodegenError> {
        let parameter_value = function_value.get_nth_param(index as u32).ok_or_else(|| {
            CodegenError::message(format!(
                "missing LLVM parameter {} for function `{}`",
                index,
                function_value.get_name().to_string_lossy()
            ))
        })?;

        let slot = self.build_alloca(parameter.type_name, &format!("{}.addr", parameter.name))?;
        map_builder(
            self.builder.build_store(slot, parameter_value),
            "store function parameter in stack slot",
        )?;

        self.bind_variable(
            parameter.name.clone(),
            StackSlot {
                pointer: slot,
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
                let _ = self.emit_expression(expression)?;
                Ok(())
            }
            IrStatement::FunctionDeclaration(function) => Err(CodegenError::message(format!(
                "nested function declaration `{}` is not supported during native emission",
                function.name
            ))),
            IrStatement::Return(value) => self.emit_return(value.as_ref()),
            IrStatement::While(while_statement) => self.emit_while_statement(while_statement),
            IrStatement::Block(block) => self.emit_block(block),
        }
    }

    fn emit_variable_declaration(
        &mut self,
        declaration: &IrVariableDeclaration,
    ) -> Result<(), CodegenError> {
        let initializer = self.emit_expression(&declaration.initializer)?;
        let initializer = self.expect_basic_value(initializer, "variable initializer")?;
        let slot =
            self.build_alloca(declaration.type_name, &format!("{}.addr", declaration.name))?;

        map_builder(
            self.builder.build_store(slot, initializer),
            "store local variable initializer",
        )?;

        self.bind_variable(
            declaration.name.clone(),
            StackSlot {
                pointer: slot,
                type_name: declaration.type_name,
            },
        )
    }

    fn emit_assignment(&mut self, assignment: &IrAssignment) -> Result<(), CodegenError> {
        let slot = self.lookup_variable(assignment.target.as_str())?;
        if slot.type_name != assignment.type_name {
            return Err(CodegenError::message(format!(
                "assignment to `{}` changed type from {} to {}",
                assignment.target, slot.type_name, assignment.type_name
            )));
        }

        let value = self.emit_expression(&assignment.value)?;
        let value = self.expect_basic_value(value, "assignment value")?;
        map_builder(
            self.builder.build_store(slot.pointer, value),
            "store assignment value",
        )?;
        Ok(())
    }

    fn emit_return(&mut self, value: Option<&IrExpression>) -> Result<(), CodegenError> {
        let expected_return_type = self.current_return_type.ok_or_else(|| {
            CodegenError::message("return statement encountered outside of a function")
        })?;

        match value {
            Some(expression) => {
                let emitted = self.emit_expression(expression)?;
                let emitted = self.expect_basic_value(emitted, "return value")?;
                map_builder(
                    self.builder.build_return(Some(&emitted)),
                    "emit function return value",
                )?;
            }
            None => {
                if expected_return_type != IrType::Void {
                    return Err(CodegenError::message(format!(
                        "non-void function attempted to return without a value ({expected_return_type})"
                    )));
                }

                map_builder(self.builder.build_return(None), "emit void return")?;
            }
        }

        self.block_terminated = true;
        Ok(())
    }

    fn emit_while_statement(
        &mut self,
        while_statement: &IrWhileStatement,
    ) -> Result<(), CodegenError> {
        let function_value = self.current_function.ok_or_else(|| {
            CodegenError::message("while statement encountered outside of a function")
        })?;

        let condition_block = self
            .context
            .append_basic_block(function_value, &self.next_label("while.cond"));
        let body_block = self
            .context
            .append_basic_block(function_value, &self.next_label("while.body"));
        let end_block = self
            .context
            .append_basic_block(function_value, &self.next_label("while.end"));

        map_builder(
            self.builder.build_unconditional_branch(condition_block),
            "branch to while condition block",
        )?;
        self.block_terminated = true;

        self.builder.position_at_end(condition_block);
        self.block_terminated = false;
        let condition_value = self.emit_expression(&while_statement.condition)?;
        let condition_value = self.expect_int_value(condition_value, "while condition")?;
        map_builder(
            self.builder
                .build_conditional_branch(condition_value, body_block, end_block),
            "emit while conditional branch",
        )?;
        self.block_terminated = true;

        self.builder.position_at_end(body_block);
        self.block_terminated = false;
        self.push_scope();
        self.emit_statements(&while_statement.body.statements)?;
        self.pop_scope()?;
        if !self.block_terminated {
            map_builder(
                self.builder.build_unconditional_branch(condition_block),
                "branch back to while condition",
            )?;
            self.block_terminated = true;
        }

        self.builder.position_at_end(end_block);
        self.block_terminated = false;
        Ok(())
    }

    fn emit_block(&mut self, block: &IrBlock) -> Result<(), CodegenError> {
        self.push_scope();
        self.emit_statements(&block.statements)?;
        self.pop_scope()?;
        Ok(())
    }

    fn emit_expression(
        &mut self,
        expression: &IrExpression,
    ) -> Result<LlvmValue<'ctx>, CodegenError> {
        match &expression.kind {
            IrExpressionKind::IntegerLiteral(value) => Ok(LlvmValue {
                value: Some(
                    self.context
                        .i64_type()
                        .const_int(*value as u64, true)
                        .into(),
                ),
                type_name: expression.type_name,
            }),
            IrExpressionKind::DoubleLiteral(value) => {
                let value = value.parse::<f64>().map_err(|error| {
                    CodegenError::message(format!(
                        "failed to parse lowered double literal `{value}` during native emission: {error}"
                    ))
                })?;

                Ok(LlvmValue {
                    value: Some(self.context.f64_type().const_float(value).into()),
                    type_name: expression.type_name,
                })
            }
            IrExpressionKind::BoolLiteral(value) => Ok(LlvmValue {
                value: Some(
                    self.context
                        .bool_type()
                        .const_int(u64::from(*value), false)
                        .into(),
                ),
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
    ) -> Result<LlvmValue<'ctx>, CodegenError> {
        let slot = self.lookup_variable(name)?;
        if slot.type_name != type_name {
            return Err(CodegenError::message(format!(
                "variable `{name}` resolved to {} instead of expected {type_name}",
                slot.type_name
            )));
        }

        let name = self.next_name("load");
        let loaded = map_builder(
            self.builder.build_load(slot.pointer, &name),
            "load local variable",
        )?;

        Ok(LlvmValue {
            value: Some(loaded),
            type_name,
        })
    }

    fn emit_unary_expression(
        &mut self,
        operator: IrUnaryOperator,
        operand: &IrExpression,
        result_type: IrType,
    ) -> Result<LlvmValue<'ctx>, CodegenError> {
        let operand = self.emit_expression(operand)?;

        match (operator, result_type) {
            (IrUnaryOperator::Negate, IrType::Int) => {
                let operand = self.expect_int_value(operand, "integer unary operand")?;
                let name = self.next_name("ineg");
                let value = map_builder(
                    self.builder.build_int_neg(operand, &name),
                    "emit integer negation",
                )?;
                Ok(LlvmValue {
                    value: Some(value.into()),
                    type_name: IrType::Int,
                })
            }
            (IrUnaryOperator::Negate, IrType::Double) => {
                let operand = self.expect_float_value(operand, "double unary operand")?;
                let name = self.next_name("fneg");
                let value = map_builder(
                    self.builder.build_float_neg(operand, &name),
                    "emit floating-point negation",
                )?;
                Ok(LlvmValue {
                    value: Some(value.into()),
                    type_name: IrType::Double,
                })
            }
            _ => Err(CodegenError::message(format!(
                "unsupported unary operator/type combination during native emission: {operator:?} {result_type}"
            ))),
        }
    }

    fn emit_binary_expression(
        &mut self,
        left: &IrExpression,
        operator: IrBinaryOperator,
        right: &IrExpression,
        result_type: IrType,
    ) -> Result<LlvmValue<'ctx>, CodegenError> {
        let left_value = self.emit_expression(left)?;
        let right_value = self.emit_expression(right)?;

        match operator {
            IrBinaryOperator::Add => {
                self.emit_numeric_binary(left_value, right_value, result_type, "add")
            }
            IrBinaryOperator::Subtract => {
                self.emit_numeric_binary(left_value, right_value, result_type, "subtract")
            }
            IrBinaryOperator::Multiply => {
                self.emit_numeric_binary(left_value, right_value, result_type, "multiply")
            }
            IrBinaryOperator::Divide => {
                self.emit_numeric_binary(left_value, right_value, result_type, "divide")
            }
            IrBinaryOperator::Less => self.emit_comparison_binary(left_value, right_value, "less"),
            IrBinaryOperator::LessEqual => {
                self.emit_comparison_binary(left_value, right_value, "less-equal")
            }
            IrBinaryOperator::Greater => {
                self.emit_comparison_binary(left_value, right_value, "greater")
            }
            IrBinaryOperator::GreaterEqual => {
                self.emit_comparison_binary(left_value, right_value, "greater-equal")
            }
            IrBinaryOperator::Equal => self.emit_equality_binary(left_value, right_value, true),
            IrBinaryOperator::NotEqual => self.emit_equality_binary(left_value, right_value, false),
        }
    }

    fn emit_numeric_binary(
        &mut self,
        left: LlvmValue<'ctx>,
        right: LlvmValue<'ctx>,
        result_type: IrType,
        operator_name: &str,
    ) -> Result<LlvmValue<'ctx>, CodegenError> {
        match result_type {
            IrType::Int => {
                let left = self.expect_int_value(left, "integer binary left operand")?;
                let right = self.expect_int_value(right, "integer binary right operand")?;
                let name = self.next_name(operator_name);
                let value = match operator_name {
                    "add" => map_builder(
                        self.builder.build_int_add(left, right, &name),
                        "emit integer add",
                    )?,
                    "subtract" => map_builder(
                        self.builder.build_int_sub(left, right, &name),
                        "emit integer subtract",
                    )?,
                    "multiply" => map_builder(
                        self.builder.build_int_mul(left, right, &name),
                        "emit integer multiply",
                    )?,
                    "divide" => map_builder(
                        self.builder.build_int_signed_div(left, right, &name),
                        "emit integer divide",
                    )?,
                    _ => {
                        return Err(CodegenError::message(format!(
                            "unsupported numeric operator `{operator_name}` for integer emission"
                        )));
                    }
                };

                Ok(LlvmValue {
                    value: Some(value.into()),
                    type_name: IrType::Int,
                })
            }
            IrType::Double => {
                let left = self.expect_float_value(left, "double binary left operand")?;
                let right = self.expect_float_value(right, "double binary right operand")?;
                let name = self.next_name(operator_name);
                let value = match operator_name {
                    "add" => map_builder(
                        self.builder.build_float_add(left, right, &name),
                        "emit double add",
                    )?,
                    "subtract" => map_builder(
                        self.builder.build_float_sub(left, right, &name),
                        "emit double subtract",
                    )?,
                    "multiply" => map_builder(
                        self.builder.build_float_mul(left, right, &name),
                        "emit double multiply",
                    )?,
                    "divide" => map_builder(
                        self.builder.build_float_div(left, right, &name),
                        "emit double divide",
                    )?,
                    _ => {
                        return Err(CodegenError::message(format!(
                            "unsupported numeric operator `{operator_name}` for double emission"
                        )));
                    }
                };

                Ok(LlvmValue {
                    value: Some(value.into()),
                    type_name: IrType::Double,
                })
            }
            _ => Err(CodegenError::message(format!(
                "numeric operator `{operator_name}` requested for non-numeric result type {result_type}"
            ))),
        }
    }

    fn emit_comparison_binary(
        &mut self,
        left: LlvmValue<'ctx>,
        right: LlvmValue<'ctx>,
        operator_name: &str,
    ) -> Result<LlvmValue<'ctx>, CodegenError> {
        let value = match left.type_name {
            IrType::Int => {
                let left = self.expect_int_value(left, "integer comparison left operand")?;
                let right = self.expect_int_value(right, "integer comparison right operand")?;
                let predicate = match operator_name {
                    "less" => IntPredicate::SLT,
                    "less-equal" => IntPredicate::SLE,
                    "greater" => IntPredicate::SGT,
                    "greater-equal" => IntPredicate::SGE,
                    _ => {
                        return Err(CodegenError::message(format!(
                            "unsupported integer comparison operator `{operator_name}`"
                        )));
                    }
                };

                let name = self.next_name("icmp");
                map_builder(
                    self.builder.build_int_compare(predicate, left, right, &name),
                    "emit integer comparison",
                )?
                .into()
            }
            IrType::Double => {
                let left = self.expect_float_value(left, "double comparison left operand")?;
                let right = self.expect_float_value(right, "double comparison right operand")?;
                let predicate = match operator_name {
                    "less" => FloatPredicate::OLT,
                    "less-equal" => FloatPredicate::OLE,
                    "greater" => FloatPredicate::OGT,
                    "greater-equal" => FloatPredicate::OGE,
                    _ => {
                        return Err(CodegenError::message(format!(
                            "unsupported double comparison operator `{operator_name}`"
                        )));
                    }
                };

                let name = self.next_name("fcmp");
                map_builder(
                    self.builder.build_float_compare(predicate, left, right, &name),
                    "emit floating-point comparison",
                )?
                .into()
            }
            _ => {
                return Err(CodegenError::message(format!(
                    "comparison operator `{operator_name}` requested for unsupported type {}",
                    left.type_name
                )));
            }
        };

        Ok(LlvmValue {
            value: Some(value),
            type_name: IrType::Bool,
        })
    }

    fn emit_equality_binary(
        &mut self,
        left: LlvmValue<'ctx>,
        right: LlvmValue<'ctx>,
        is_equal: bool,
    ) -> Result<LlvmValue<'ctx>, CodegenError> {
        let value = match left.type_name {
            IrType::Int | IrType::Bool => {
                let left = self.expect_int_value(left, "integer equality left operand")?;
                let right = self.expect_int_value(right, "integer equality right operand")?;
                let predicate = if is_equal {
                    IntPredicate::EQ
                } else {
                    IntPredicate::NE
                };
                let name = self.next_name("icmp.eq");
                map_builder(
                    self.builder.build_int_compare(predicate, left, right, &name),
                    "emit integer equality comparison",
                )?
                .into()
            }
            IrType::Double => {
                let left = self.expect_float_value(left, "double equality left operand")?;
                let right = self.expect_float_value(right, "double equality right operand")?;
                let predicate = if is_equal {
                    FloatPredicate::OEQ
                } else {
                    FloatPredicate::ONE
                };
                let name = self.next_name("fcmp.eq");
                map_builder(
                    self.builder.build_float_compare(predicate, left, right, &name),
                    "emit floating-point equality comparison",
                )?
                .into()
            }
            _ => {
                return Err(CodegenError::message(format!(
                    "equality comparison requested for unsupported type {}",
                    left.type_name
                )));
            }
        };

        Ok(LlvmValue {
            value: Some(value),
            type_name: IrType::Bool,
        })
    }

    fn emit_call_expression(
        &mut self,
        callee: &str,
        arguments: &[IrExpression],
        return_type: IrType,
    ) -> Result<LlvmValue<'ctx>, CodegenError> {
        let function_value = self.lookup_function(callee)?;
        let mut emitted_arguments = Vec::with_capacity(arguments.len());

        for argument in arguments {
            let value = self.emit_expression(argument)?;
            let value = self.expect_basic_value(value, "call argument")?;
            emitted_arguments.push(value.into());
        }

        let name = self.next_name("call");
        let call_site = map_builder(
            self.builder.build_call(function_value, &emitted_arguments, &name),
            "emit direct function call",
        )?;

        if return_type == IrType::Void {
            Ok(LlvmValue {
                value: None,
                type_name: IrType::Void,
            })
        } else {
            let value = call_site.try_as_basic_value().left().ok_or_else(|| {
                CodegenError::message(format!(
                    "call to `{callee}` did not produce a first-class value for return type {return_type}"
                ))
            })?;

            Ok(LlvmValue {
                value: Some(value),
                type_name: return_type,
            })
        }
    }

    fn emit_cast_expression(
        &mut self,
        expression: &IrExpression,
        target_type: IrType,
    ) -> Result<LlvmValue<'ctx>, CodegenError> {
        let value = self.emit_expression(expression)?;

        match (value.type_name, target_type) {
            (IrType::Int, IrType::Double) => {
                let value = self.expect_int_value(value, "cast operand")?;
                let name = self.next_name("sitofp");
                let cast = map_builder(
                    self.builder.build_signed_int_to_float(
                        value,
                        self.context.f64_type(),
                        &name,
                    ),
                    "emit int-to-double cast",
                )?;

                Ok(LlvmValue {
                    value: Some(cast.into()),
                    type_name: IrType::Double,
                })
            }
            _ => Err(CodegenError::message(format!(
                "unsupported cast during native emission: {} to {}",
                value.type_name, target_type
            ))),
        }
    }

    fn function_type(
        &self,
        parameters: &[IrType],
        return_type: IrType,
    ) -> Result<FunctionType<'ctx>, CodegenError> {
        let parameters = parameters
            .iter()
            .map(|parameter| self.basic_type(*parameter).map(Into::into))
            .collect::<Result<Vec<BasicMetadataTypeEnum<'ctx>>, CodegenError>>()?;

        match return_type {
            IrType::Void => Ok(self.context.void_type().fn_type(&parameters, false)),
            IrType::Int => Ok(self.context.i64_type().fn_type(&parameters, false)),
            IrType::Double => Ok(self.context.f64_type().fn_type(&parameters, false)),
            IrType::Bool => Ok(self.context.bool_type().fn_type(&parameters, false)),
        }
    }

    fn basic_type(&self, type_name: IrType) -> Result<BasicTypeEnum<'ctx>, CodegenError> {
        match type_name {
            IrType::Int => Ok(self.context.i64_type().into()),
            IrType::Double => Ok(self.context.f64_type().into()),
            IrType::Bool => Ok(self.context.bool_type().into()),
            IrType::Void => Err(CodegenError::message(
                "attempted to materialize a basic LLVM type for void",
            )),
        }
    }

    fn integer_type(&self, type_name: IrType) -> Result<inkwell::types::IntType<'ctx>, CodegenError> {
        match type_name {
            IrType::Int => Ok(self.context.i64_type()),
            IrType::Bool => Ok(self.context.bool_type()),
            IrType::Double | IrType::Void => Err(CodegenError::message(format!(
                "expected integer LLVM type, found {type_name}"
            ))),
        }
    }

    fn build_alloca(
        &self,
        type_name: IrType,
        name: &str,
    ) -> Result<PointerValue<'ctx>, CodegenError> {
        map_builder(
            self.builder.build_alloca(self.basic_type(type_name)?, name),
            "allocate stack slot",
        )
    }

    fn expect_basic_value(
        &self,
        value: LlvmValue<'ctx>,
        context: &str,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        value.value.ok_or_else(|| {
            CodegenError::message(format!(
                "expected a value-producing expression while emitting {context}, but got `{}`",
                value.type_name
            ))
        })
    }

    fn expect_int_value(
        &self,
        value: LlvmValue<'ctx>,
        context: &str,
    ) -> Result<IntValue<'ctx>, CodegenError> {
        let value = self.expect_basic_value(value, context)?;
        Ok(value.into_int_value())
    }

    fn expect_float_value(
        &self,
        value: LlvmValue<'ctx>,
        context: &str,
    ) -> Result<FloatValue<'ctx>, CodegenError> {
        let value = self.expect_basic_value(value, context)?;
        Ok(value.into_float_value())
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) -> Result<(), CodegenError> {
        self.scopes.pop().map(|_| ()).ok_or_else(|| {
            CodegenError::message("attempted to pop an empty scope stack during native emission")
        })
    }

    fn bind_variable(&mut self, name: String, slot: StackSlot<'ctx>) -> Result<(), CodegenError> {
        let scope = self.scopes.last_mut().ok_or_else(|| {
            CodegenError::message("attempted to bind a variable without an active scope")
        })?;

        scope.insert(name, slot);
        Ok(())
    }

    fn lookup_variable(&self, name: &str) -> Result<StackSlot<'ctx>, CodegenError> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
            .ok_or_else(|| {
                CodegenError::message(format!("unknown variable `{name}` in TS-Native IR"))
            })
    }

    fn lookup_function(&self, name: &str) -> Result<FunctionValue<'ctx>, CodegenError> {
        self.function_values.get(name).copied().ok_or_else(|| {
            CodegenError::message(format!("unknown function `{name}` in TS-Native IR"))
        })
    }

    fn next_name(&mut self, prefix: &str) -> String {
        let name = format!("{prefix}.{}", self.name_counter);
        self.name_counter += 1;
        name
    }

    fn next_label(&mut self, prefix: &str) -> String {
        let label = format!("{prefix}.{}", self.label_counter);
        self.label_counter += 1;
        label
    }
}

fn map_builder<T>(result: Result<T, BuilderError>, context: &str) -> Result<T, CodegenError> {
    result.map_err(|error| CodegenError::message(format!("failed to {context}: {error}")))
}

fn builtin_runtime_argument_type(name: &str) -> IrType {
    match name {
        RUNTIME_PRINT_INT => IrType::Int,
        RUNTIME_PRINT_DOUBLE => IrType::Double,
        RUNTIME_PRINT_BOOL => IrType::Int,
        _ => IrType::Void,
    }
}
