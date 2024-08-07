use std::{
    collections::HashMap,
    error::Error,
    fmt::Display,
    io::{BufRead, Write},
};

use commands::{Command, CommandExecutor, StatusCode};
use operators::execute_operator_expression;

use crate::{
    constants::UNDERSCORE,
    parser::{
        code_blocks::CodeBlock,
        expressions::{CaptureExitCode, Expression},
        functions::Function,
        literals::StringLiteral,
        statements::{Assignment, Statement},
    },
};

mod builtins;
pub mod commands;
mod operators;

#[derive(Debug)]
pub struct ExecutionError {
    pub message: String,
}

impl ExecutionError {
    pub(crate) fn new(message: String) -> Self {
        Self { message }
    }
}

impl From<&str> for ExecutionError {
    fn from(value: &str) -> Self {
        ExecutionError::new(value.to_owned())
    }
}

impl From<String> for ExecutionError {
    fn from(value: String) -> Self {
        ExecutionError::new(value)
    }
}

impl Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for ExecutionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Value {
    #[default]
    Void,
    String(String),
    Integer(i32),
    Boolean(bool),
    Command(commands::Command),
    Tuple(Vec<Value>),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Void => f.write_str("void")?,
            Value::String(data) => f.write_str(data)?,
            Value::Integer(data) => data.fmt(f)?,
            Value::Boolean(data) => data.fmt(f)?,
            Value::Command(data) => data.fmt(f)?,
            Value::Tuple(data) => {
                f.write_str("(")?;
                let mut first = true;
                for element in data {
                    if !first {
                        f.write_str(",")?;
                    } else {
                        first = false;
                    }
                    element.fmt(f)?;
                }
                f.write_str(")")?;
            }
        };

        return Ok(());
    }
}

struct ExecutorContext {
    pub command_executor: Box<dyn CommandExecutor>,
    pub stdin: Box<dyn BufRead>,
    pub stdout: Box<dyn Write>,
    pub stderr: Box<dyn Write>,
}

struct ExecutorStack {
    pub variables: HashMap<String, Value>,
    pub functions: HashMap<String, Function>,
}

pub struct Executor {
    context: ExecutorContext,
}

impl Executor {
    pub fn new<
        T: CommandExecutor + 'static,
        Stdin: BufRead + 'static,
        Stdout: Write + 'static,
        Stderr: Write + 'static,
    >(
        command_executor: T,
        stdin: Stdin,
        stdout: Stdout,
        stderr: Stderr,
    ) -> Self {
        Executor {
            context: ExecutorContext {
                command_executor: Box::new(command_executor),
                stdin: Box::new(stdin),
                stdout: Box::new(stdout),
                stderr: Box::new(stderr),
            },
        }
    }

    pub fn execute(&mut self, root: &CodeBlock) -> Result<(), ExecutionError> {
        let mut stack = ExecutorStack {
            functions: HashMap::new(),
            variables: HashMap::new(),
        };
        self.execute_block(root, &mut stack)?;
        return Ok(());
    }

    fn execute_block(
        &mut self,
        code: &CodeBlock,
        stack: &mut ExecutorStack,
    ) -> Result<(), ExecutionError> {
        for function in &code.functions {
            stack
                .functions
                .insert(function.name.value.to_owned(), function.clone());
        }
        for statement in &code.statements {
            self.execute_statement(statement, stack)?;
        }
        return Ok(());
    }

    fn execute_statement(
        &mut self,
        statement: &Statement,
        stack: &mut ExecutorStack,
    ) -> Result<(), ExecutionError> {
        match statement {
            Statement::Assignment(assignment, expression) => {
                let result = self.execute_expression(expression, stack)?;
                match assignment {
                    Assignment::Simple(identifier) => {
                        assign_variable(result, &identifier.value, stack)?;
                    }
                    Assignment::Tuple(identifiers) => {
                        let Value::Tuple(result) = result else {
                            return Err(
                                "Can't use a tuple assignment with a non-tuple value".into()
                            );
                        };

                        if identifiers.len() > result.len() {
                            return Err("Not enough values in tuple to fill assignment".into());
                        }

                        for (identifier, result) in identifiers.iter().zip(result) {
                            assign_variable(result, &identifier.value, stack)?;
                        }
                    }
                }
            }
            Statement::Declaration(assignment, expression) => {
                let result = self.execute_expression(expression, stack)?;
                match assignment {
                    Assignment::Simple(identifier) => {
                        declare_variable(result, &identifier.value, stack)?;
                    }
                    Assignment::Tuple(identifiers) => {
                        let Value::Tuple(result) = result else {
                            return Err(
                                "Can't use a tuple assignment with a non-tuple value".into()
                            );
                        };

                        if identifiers.len() > result.len() {
                            return Err("Not enough values in tuple to fill assignment".into());
                        }

                        for (identifier, result) in identifiers.iter().zip(result) {
                            declare_variable(result, &identifier.value, stack)?;
                        }
                    }
                }
            }
            Statement::Expression(expression) => {
                let value = self.execute_expression(expression, stack)?;

                if let Value::Void = value {
                } else {
                    if let Err(err) = writeln!(&mut self.context.stdout, "{:}", value) {
                        return Err(ExecutionError::new(format!(
                            "Error writing to stdout: {err}"
                        )));
                    }

                    if let Err(err) = self.context.stdout.flush() {
                        return Err(ExecutionError::new(format!("Error flushing stdout: {err}")));
                    }
                }
            }
        };

        return Ok(());
    }

    fn execute_expression(
        &mut self,
        expression: &Expression,
        stack: &mut ExecutorStack,
    ) -> Result<Value, ExecutionError> {
        match expression {
            Expression::StringLiteral(literal) => {
                Ok(Value::String(evaluate_string_literal(literal, stack)?))
            }
            Expression::BooleanLiteral(boolean) => Ok(Value::Boolean(*boolean)),
            Expression::IntegerLiteral(integer) => Ok(Value::Integer(*integer)),
            Expression::Variable(variable_name) => resolve_variable(&variable_name.value, stack),
            Expression::Tuple(elements) => Ok(Value::Tuple(
                elements
                    .iter()
                    .map(|expression| self.execute_expression(expression, stack))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Expression::Command(command) => {
                let mut command = command.iter();
                return Ok(Value::Command(Command::new(
                    evaluate_string_literal(
                        command.next().ok_or::<ExecutionError>(
                            "Command must include at least one element".into(),
                        )?,
                        stack,
                    )?,
                    command
                        .map(|argument| evaluate_string_literal(argument, stack))
                        .collect::<Result<Vec<_>, _>>()?,
                )));
            }
            Expression::Operation(left, operator, right) => {
                let left = self.execute_expression(&left, stack)?;
                let right = self.execute_expression(&right, stack)?;
                return execute_operator_expression(&operator, left, right);
            }
            Expression::If(blocks, else_block) => {
                for (condition, block) in blocks {
                    let condition_result = self.execute_expression(condition, stack)?;
                    if let Value::Boolean(result) = condition_result {
                        if result {
                            self.execute_block(block, stack)?;
                            return Ok(Value::Void);
                        }
                    } else {
                        return Err(ExecutionError::new(
                            "If statement condition must evaluate to a boolean".to_owned(),
                        ));
                    }
                }

                if let Some(else_block) = else_block {
                    self.execute_block(else_block, stack)?;
                }
                return Ok(Value::Void);
            }
            Expression::Execute(expression, capture_exit_code) => {
                if let Value::Command(command) = self.execute_expression(expression, stack)? {
                    let result = self.context.command_executor.run(&command).map_err(|err| {
                        ExecutionError::new(format!("Error running command: {:}", err))
                    })?;

                    let (return_value, exit_code) = match result.status_code {
                        StatusCode::Terminated => {
                            return Err(ExecutionError::new(format!("Command was terminated")))
                        }
                        StatusCode::Value(code) => (
                            Value::Tuple(vec![
                                Value::String(result.stdout),
                                Value::String(result.stderr),
                            ]),
                            code,
                        ),
                    };

                    match capture_exit_code {
                        Some(CaptureExitCode::Assignment(identifier)) => {
                            assign_variable(
                                Value::Integer(exit_code.into()),
                                &identifier.value,
                                stack,
                            )?;
                        }
                        Some(CaptureExitCode::Declaration(identifier)) => {
                            declare_variable(
                                Value::Integer(exit_code.into()),
                                &identifier.value,
                                stack,
                            )?;
                        }
                        None => {
                            if exit_code != 0 {
                                return Err(ExecutionError::new(format!(
                                    "Command returned non-zero exit code: ({:})",
                                    exit_code
                                )));
                            }
                        }
                    }

                    return Ok(return_value);
                }

                return Err(ExecutionError::new("Must execute a command".to_owned()));
            }
            Expression::FunctionCall(name, args) => {
                let args = args
                    .iter()
                    .map(|arg| self.execute_expression(arg, stack))
                    .collect::<Result<Vec<_>, _>>()?;

                // Remove function from stack when calling it to avoid double borrowing, means
                // recursion won't work, but that needs stack frames to work anyway.
                if let Some(function) = stack.functions.remove(&name.value) {
                    self.execute_block(&function.code, stack)?;
                    stack.functions.insert(name.value.to_owned(), function);
                    Ok(Value::Void)
                } else {
                    builtins::call_builtin(&name.value, args.as_slice(), &mut self.context)
                }
            }
            Expression::Get(base_expression, index) => {
                let Value::Tuple(mut values) = self.execute_expression(base_expression, stack)?
                else {
                    return Err("Cannot use get expression on non-tuple value".into());
                };

                let len = values.len();
                let result = values.get_mut(*index as usize).ok_or(format!(
                    "Cannot get element at index {:} because tuple only has {:} elements",
                    index, len
                ))?;

                return Ok(core::mem::take(result));
            }
        }
    }
}

fn evaluate_string_literal(
    literal: &StringLiteral,
    stack: &ExecutorStack,
) -> Result<String, ExecutionError> {
    let mut result = String::new();
    for (prefix, identifier) in &literal.parts {
        result += &prefix;
        let Value::String(variable_value) = resolve_variable(&identifier.value, stack)? else {
            return Err("Template variable in strings must resolve to a string".into());
        };
        result += &variable_value;
    }
    result += &literal.end;
    Ok(result)
}

fn resolve_variable(variable_name: &str, stack: &ExecutorStack) -> Result<Value, ExecutionError> {
    Ok(stack
        .variables
        .get(variable_name)
        .ok_or(ExecutionError::new("Variable not found".to_owned()))?
        .clone())
}

fn assign_variable(
    value: Value,
    variable_name: &str,
    stack: &mut ExecutorStack,
) -> Result<(), ExecutionError> {
    if variable_name == UNDERSCORE {
        return Ok(());
    }

    if let Some(variable) = stack.variables.get_mut(variable_name) {
        *variable = value;
    } else {
        return Err("Couldn't find variable".into());
    }

    Ok(())
}

fn declare_variable(
    value: Value,
    variable_name: &str,
    stack: &mut ExecutorStack,
) -> Result<(), ExecutionError> {
    if variable_name == UNDERSCORE {
        return Ok(());
    }
    if let Some(_) = stack.variables.get(variable_name) {
        return Err(ExecutionError::new("Variable already exists".into()));
    } else {
        stack.variables.insert(variable_name.to_owned(), value);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::{self, Read};

    use commands::CommandResult;
    use mockall::{mock, predicate};

    use crate::parser::{
        functions::Function, literals::StringLiteral, operators::Operator, statements::Assignment,
    };

    use super::*;

    mock! {
        pub CommandExecutor {}

        impl CommandExecutor for CommandExecutor {
            fn run(&self, command: &Command) -> io::Result<CommandResult>;
        }
    }

    mock! {
        pub In {}

        impl Read for In {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
        }

        impl BufRead for In {
            fn fill_buf<'a>(&'a mut self) -> io::Result<&'a [u8]>;
            fn consume(&mut self, amt: usize);
        }
    }

    mock! {
        pub Out {}

        impl Write for Out {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize>;
            fn flush(&mut self) -> io::Result<()>;
        }
    }

    #[test]
    fn should_execute_valid_syntax_tree() {
        let syntax_tree = CodeBlock {
            functions: vec![Function {
                name: "main".into(),
                arguments: vec![],
                code: CodeBlock {
                    functions: vec![],
                    statements: vec![
                        Statement::Declaration(
                            Assignment::Simple("template".into()),
                            Expression::StringLiteral("cheese".into()),
                        ),
                        Statement::Declaration(
                            Assignment::Simple("test_identifier".into()),
                            Expression::StringLiteral(StringLiteral::new(
                                vec![("Blue \"".to_owned(), "template".into())],
                                "\" and rice!".to_owned(),
                            )),
                        ),
                        Statement::Expression(Expression::If(
                            vec![(
                                Expression::Operation(
                                    Box::new(Expression::Operation(
                                        Box::new(Expression::IntegerLiteral(1)),
                                        Operator::Addition,
                                        Box::new(Expression::IntegerLiteral(1)),
                                    )),
                                    Operator::Equals,
                                    Box::new(Expression::IntegerLiteral(2)),
                                ),
                                CodeBlock {
                                    functions: vec![],
                                    statements: vec![Statement::Expression(Expression::Variable(
                                        "test_identifier".into(),
                                    ))],
                                },
                            )],
                            None,
                        )),
                        Statement::Assignment(
                            Assignment::Simple("_".into()),
                            Expression::Execute(
                                Box::new(Expression::Command(vec![
                                    "echo".into(),
                                    "something".into(),
                                ])),
                                None,
                            ),
                        ),
                    ],
                },
            }],
            statements: vec![Statement::Expression(Expression::FunctionCall(
                "main".into(),
                vec![],
            ))],
        };

        let mut mock_command_executor = MockCommandExecutor::new();
        let mock_stdin = MockIn::new();
        let mut mock_stdout = MockOut::new();
        let mock_stderr = MockOut::new();

        mock_command_executor
            .expect_run()
            .with(predicate::eq(Command::new(
                "echo".to_owned(),
                vec!["something".to_owned()],
            )))
            .return_once(|_| {
                Ok(CommandResult {
                    status_code: 0.into(),
                    stdout: "something".to_owned(),
                    stderr: "".to_owned(),
                })
            })
            .once();
        mock_stdout
            .expect_write()
            .once()
            .with(predicate::eq("Blue \"cheese\" and rice!".as_bytes()))
            .returning(|data| Ok(data.len()));
        mock_stdout
            .expect_write()
            .once()
            .with(predicate::eq("\n".as_bytes()))
            .returning(|data| Ok(data.len()));
        mock_stdout.expect_flush().return_once(|| Ok(())).once();

        let mut executor =
            Executor::new(mock_command_executor, mock_stdin, mock_stdout, mock_stderr);

        executor.execute(&syntax_tree).unwrap();
    }
}
