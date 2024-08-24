#[cfg(test)]
mod tests {
    use insta::assert_yaml_snapshot;
    use mockall::{mock, predicate};
    use nash::*;
    use serde::Serialize;
    use std::io::{BufReader, Result as IoResult};

    mock! {
        pub CommandExecutor {}

        impl CommandExecutor for CommandExecutor {
            fn run(&self, command: &Command) -> IoResult<CommandResult>;
        }
    }

    #[derive(Serialize)]
    struct CodeOutput {
        stdout: String,
        stderr: String,
        error: Option<NashError>,
    }

    fn run_code(script: &str) -> CodeOutput {
        run_code_with_setup(script, |_| {})
    }

    fn run_code_with_setup<F: FnOnce(&mut MockCommandExecutor)>(
        script: &str,
        setup: F,
    ) -> CodeOutput {
        let input = Vec::new();
        let mut mock_command_executor = MockCommandExecutor::new();
        let mut mock_in = BufReader::new(&input[..]);
        let mut mock_out = Vec::new();
        let mut mock_err = Vec::new();

        setup(&mut mock_command_executor);

        let error = {
            let mut executor = Executor::new(
                mock_command_executor,
                &mut mock_in,
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
        |mock_command_executor| {
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
  array.pop();
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
array.push(1);
array.push(2);
array.push(3);
out(array.fmt());
"#
    );

    nash_test!(
        should_not_access_outer_scopes_from_function,
        r#"
var test = "hi";
func my_function() {
  out(test);
}
my_function();
"#
    );

    nash_test!(
        should_return_value_from_function,
        r#"
func my_function(): string  {
    return "value";
}
out(my_function());
"#
    );

    nash_test!(
        should_support_returning_from_nested_blocks,
        r#"
func my_function(): string  {
    if true {
        return "value";
    };
}

out(my_function());
"#
    );

    nash_test!(
        should_not_support_returning_from_root_block,
        r#"
return 123;
"#
    );

    nash_test!(
        should_error_when_function_called_with_argument_argument_of_wrong_type,
        r#"
func test(arg: string) {}
test(123);
"#
    );

    nash_test!(
        should_error_when_assigning_void_to_a_variable,
        r#"
func test() {}
var my_variable = test();
"#
    );

    nash_test!(
        should_error_when_declaring_a_variable_of_type_void,
        r#"
var test: void;
"#
    );

    nash_test!(
        should_error_when_creating_a_function_with_an_argument_of_void,
        r#"
func test(arg: void) {}
"#
    );

    nash_test!(
        should_break_from_while_loop,
        r#"
var index = 0;
while true {
  index = index + 1;
  if index == 5 {
    break;
  };
  out(index.fmt());
};
"#
    );

    nash_test!(
        should_continue_in_while_loop,
        r#"
var index = 0;
while index < 5 {
    index = index + 1;
    if index == 2 {
      continue;
    };
    out(index.fmt());
};
"#
    );

    nash_test!(
        should_break_in_for_loop,
        r#"
for item in [1, 2, 3, 4, 5] {
    if item == 3 {
      continue;
    };
    out(item.fmt());
};
"#
    );

    nash_test!(
        should_error_if_break_called_directly_from_root,
        r#"
if true {
  break;
};
"#
    );

    nash_test!(
        should_error_if_continue_called_directly_from_root,
        r#"
{
  continue;
};
"#
    );

    nash_test!(
        should_error_if_break_called_directly_from_function,
        r#"
func main() {
  if false {
  } else {
    break;
  };
}

main();
"#
    );

    nash_test!(
        should_error_if_continue_called_directly_from_function,
        r#"
func main() {
  continue;
}

main();
"#
    );
}
