use std::{
    collections::HashMap,
    error::Error,
    fmt::Display,
    io::{BufRead, Write},
};

use commands::{Command, CommandExecutor, StatusCode};
use operators::execute_operator_expression;

use crate::parser::{
    code_blocks::CodeBlock, expressions::Expression, functions::Function, literals::StringLiteral,
    statements::Statement,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Void,
    String(String),
    Integer(i32),
    Boolean(bool),
    Command(commands::Command),
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
            Statement::Assignment(variable_name, expression) => {
                let result = self.execute_expression(expression, stack)?;
                if let Some(variable) = stack.variables.get_mut(&variable_name.value) {
                    *variable = result;
                } else {
                    return Err(ExecutionError::new("Couldn't find variable".into()));
                }
            }
            Statement::Declaration(variable_name, expression) => {
                if let Some(_) = stack.variables.get(&variable_name.value) {
                    return Err(ExecutionError::new("Variable already exists".into()));
                } else {
                    let result = self.execute_expression(expression, stack)?;
                    stack.variables.insert(variable_name.value.clone(), result);
                }
            }
            Statement::Expression(expression) => {
                self.execute_expression(expression, stack)?;
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
            Expression::Execute(expression) => {
                if let Value::Command(command) = self.execute_expression(expression, stack)? {
                    let result = self.context.command_executor.run(&command).map_err(|err| {
                        ExecutionError::new(format!("Error running command: {:}", err))
                    })?;

                    return match result.status_code {
                        StatusCode::Error(code) => Err(ExecutionError::new(format!(
                            "Command exited with non-zero exit code ({:})",
                            code
                        ))),
                        StatusCode::Terminated => {
                            Err(ExecutionError::new(format!("Command was terminated",)))
                        }
                        StatusCode::Ok => Ok(Value::String(result.stdout)),
                    };
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

#[cfg(test)]
mod tests {
    use std::io::{self, Read};

    use commands::CommandOutput;
    use mockall::{mock, predicate};

    use crate::parser::{functions::Function, literals::StringLiteral, operators::Operator};

    use super::*;

    mock! {
        pub CommandExecutor {}

        impl CommandExecutor for CommandExecutor {
            fn run(&self, command: &Command) -> io::Result<CommandOutput>;
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
                            "template".into(),
                            Expression::StringLiteral("cheese".into()),
                        ),
                        Statement::Declaration(
                            "test_identifier".into(),
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
                                    statements: vec![Statement::Expression(
                                        Expression::FunctionCall(
                                            "out".into(),
                                            vec![Expression::Variable("test_identifier".into())],
                                        ),
                                    )],
                                },
                            )],
                            None,
                        )),
                        Statement::Expression(Expression::Execute(Box::new(Expression::Command(
                            vec!["echo".into(), "something".into()],
                        )))),
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
                Ok(CommandOutput {
                    status_code: 0.into(),
                    stdout: "something".to_owned(),
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
