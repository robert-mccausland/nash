#[cfg(test)]
mod tests {
    use insta::assert_yaml_snapshot;
    use mockall::{mock, predicate};
    use nash::*;
    use serde::Serialize;
    use std::io::{BufRead, Read, Result as IoResult, Write};

    mock! {
        pub CommandExecutor {}

        impl CommandExecutor for CommandExecutor {
            fn run(&self, command: &Command) -> IoResult<CommandResult>;
        }
    }

    mock! {
        pub In {}

        impl Read for In {
            fn read(&mut self, buf: &mut [u8]) -> IoResult<usize>;
        }

        impl BufRead for In {
            fn fill_buf<'a>(&'a mut self) -> IoResult<&'a [u8]>;
            fn consume(&mut self, amt: usize);
        }
    }

    mock! {
        pub Out {}

        impl Write for Out {
            fn write(&mut self, buf: &[u8]) -> IoResult<usize>;
            fn flush(&mut self) -> IoResult<()>;
        }
    }

    #[derive(Serialize)]
    struct CodeOutput {
        stdout: String,
        stderr: String,
        error: Option<NashError>,
    }

    fn run_code(script: &str) -> CodeOutput {
        run_code_with_setup(script, |_, _| {})
    }

    fn run_code_with_setup<F: FnOnce(&mut MockCommandExecutor, &mut MockIn)>(
        script: &str,
        setup: F,
    ) -> CodeOutput {
        let mut mock_command_executor = MockCommandExecutor::new();
        let mut mock_in = MockIn::new();
        let mut mock_out = Vec::new();
        let mut mock_err = Vec::new();

        setup(&mut mock_command_executor, &mut mock_in);

        let error = {
            let mut executor = Executor::new(
                mock_command_executor,
                mock_in,
                &mut mock_out,
                &mut mock_err,
                ExecutorOptions::default(),
            );

            nash::execute(&mut script.as_bytes(), &mut executor).err()
        };

        return CodeOutput {
            stdout: String::from_utf8(mock_out).unwrap(),
            stderr: String::from_utf8(mock_err).unwrap(),
            error,
        };
    }

    macro_rules! nash_test {
        ($name:ident, $code:expr) => {
            #[test]
            fn $name() {
                assert_yaml_snapshot!(run_code($code));
            }
        };

        ($name:ident, $code:expr, $setup:expr) => {
            #[test]
            fn $name() {
                assert_yaml_snapshot!(run_code_with_setup($code, $setup));
            }
        };
    }

    nash_test!(
        should_execute_valid_file,
        r#"
# Comments are fun!
func main() {
    var test_identifier = "Blue \"cheese\" and rice!";
    if 1 + 1 == 2 {
    out(test_identifier);
    };

    exec `echo something`;
}

main();
"#,
        |mock_command_executor, _| {
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
        }
    );

    nash_test!(
        should_fail_to_mutate_array_in_use,
        r#"
# Array example
var array = ["first", "second", "third"];
for value in array {
  pop(array);
};
"#
    );
    nash_test!(
        should_error_when_creating_array_with_inconsistent_types,
        r#"
var variable = ["test", 123];
"#
    );

    nash_test!(
        should_error_when_creating_an_empty_array_with_no_type,
        r#"
var variable = [];
"#
    );
    nash_test!(
        should_error_when_assigning_variable_to_wrong_type,
        r#"
var variable = "test";
variable = 42;
"#
    );

    nash_test!(
        should_error_when_resolving_uninitialized_variable,
        r#"
var variable: string;
out(variable);
"#
    );

    nash_test!(
        should_error_when_uninitialized_variable_defined_without_type,
        r#"
var variable;
"#
    );

    nash_test!(
        should_generate_arrays_correctly,
        r#"
var array = [0];
push(array, 1);
push(array, 2);
push(array, 3);
out(fmt(array));
"#
    );
}
