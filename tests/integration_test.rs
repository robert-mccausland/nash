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
        exit_code: u8,
    }

    fn run_code(script: &str) -> CodeOutput {
        run_code_with_setup(script, "", |_| {})
    }

    fn run_code_with_setup<F: FnOnce(&mut MockCommandExecutor)>(
        script: &str,
        input: &str,
        setup: F,
    ) -> CodeOutput {
        let input = input.as_bytes().to_vec();
        let mut mock_command_executor = MockCommandExecutor::new();
        let mut mock_in = BufReader::new(&input[..]);
        let mut mock_out = Vec::new();
        let mut mock_err = Vec::new();

        setup(&mut mock_command_executor);

        let result = {
            let mut executor = Executor::new(
                mock_command_executor,
                &mut mock_in,
                &mut mock_out,
                &mut mock_err,
                ExecutorOptions::default(),
            );

            nash::execute(&mut script.as_bytes(), &mut executor)
        };

        let exit_code = match &result {
            Ok(output) => output.exit_code(),
            Err(error) => error.exit_code(),
        };

        return CodeOutput {
            stdout: String::from_utf8(mock_out).unwrap(),
            stderr: String::from_utf8(mock_err).unwrap(),
            error: result.err(),
            exit_code,
        };
    }

    macro_rules! nash_test {
        ($name:ident, $code:expr) => {
            #[test]
            fn $name() {
                assert_yaml_snapshot!(run_code($code));
            }
        };

        ($name:ident, $code:expr, $input:expr) => {
            #[test]
            fn $name() {
                assert_yaml_snapshot!(run_code_with_setup($code, $input, |_| {}));
            }
        };

        ($name:ident, $code:expr, $input:expr, $setup:expr) => {
            #[test]
            fn $name() {
                assert_yaml_snapshot!(run_code_with_setup($code, $input, $setup));
            }
        };
    }

    nash_test!(
        should_execute_valid_file,
        r#"
# Comments are fun!
func main() {
    var test_identifier = "Blue \"cheese\" and rice!";
    if (1 + 1) == 2 {
    out(test_identifier);
    };

    exec `echo something`;
}

main();
"#,
        "",
        |mock_command_executor| {
            mock_command_executor
                .expect_run()
                .with(predicate::eq(Command::new(
                    "echo".to_owned(),
                    vec!["something".to_owned()],
                )))
                .return_once(|_| Ok(CommandResult::new(0, "something")))
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

    nash_test!(
        should_be_able_to_chain_accessors,
        r#"
var value = ("string", [1, 2, 3]);
out(value.1.fmt());
"#
    );

    nash_test!(
        should_be_able_to_return_with_an_exit_code,
        r#"
exit 69;
"#
    );

    nash_test!(
        should_fail_if_exit_used_with_non_integer,
        r#"
exit "test";
"#
    );

    nash_test!(
        should_fail_if_exit_used_with_out_of_range_value,
        r#"
exit 1000;
"#
    );

    nash_test!(
        should_be_able_to_return_from_nested_blocks,
        r#"
func main() {
  for value in [1, 2, 3, 4, 5] {
    out(value.fmt());
    if value == 4 {
      exit value;
    };
  };
}

main();
"#
    );

    nash_test!(
        should_not_run_any_code_after_exit,
        r#"
exit 0;
out("This should not be printed!");
"#
    );

    nash_test!(
        strings_should_be_equal_if_contents_is_the_same,
        r#"
var my_value = "my_string" == "my_string";
out(my_value.fmt());
"#
    );

    nash_test!(
        should_accept_input_using_read,
        r#"
var input1 = read();
var input2 = read();
var input3 = read();
out((input1, input2, input3).fmt());
"#,
        "test_input_1\ntest_input_2\ntest_input_3\n"
    );

    nash_test!(
        should_strip_trailing_carriage_return_from_input,
        r#"
var test_input = read();
out(test_input.fmt());
"#,
        "my_fancy_input\r\nextra_stuff_here"
    );

    nash_test!(
        should_allow_expressions_inside_string_templates,
        r#"
var variable = 123;
out("My variable is: ${variable.fmt()}!");
"#
    );

    nash_test!(
        should_allow_brackets_to_be_used_to_wrap_expressions,
        r#"
var string = (1 + 2).fmt();
out(string);
"#
    );

    nash_test!(
        should_allow_singleton_tuples_by_using_trailing_comma,
        r#"
var string = (1 + 2,).fmt();
out(string);
"#
    );

    nash_test!(
        should_allow_arrays_with_trailing_commas,
        r#"
out([1, 2, 3, 4,].fmt());
"#
    );

    nash_test!(
        should_handle_minus_operator,
        r#"
out((10 - 5).fmt());
"#
    );

    nash_test!(
        should_handle_multiply_operator,
        r#"
out((6 * 7).fmt());
"#
    );

    nash_test!(
        should_handle_divide_operator,
        r#"
out((30 / 5).fmt());
"#
    );

    nash_test!(
        should_handle_remainder_operator,
        r#"
out((69 % 30).fmt());
"#
    );

    nash_test!(
        should_handle_and_operator,
        r#"
if true && true {
  out("yes!");
};
"#
    );

    nash_test!(
        should_handle_or_operator,
        r#"
if false || true {
  out("yes!");
};
"#
    );

    nash_test!(
        should_allow_chaining_commutative_operators,
        r#"
out((1 + 2 + 3 + 4 - 10).fmt());
"#
    );

    nash_test!(
        should_not_allow_chaining_non_commutative_operators,
        r#"
out((1 + 2 + 3 + 4 * 10).fmt());
"#
    );

    nash_test!(
        should_error_if_command_returns_non_zero_exit_code,
        r#"
exec `my_command`;
out(code.fmt());
"#,
        "",
        |executor| {
            executor
                .expect_run()
                .with(predicate::eq::<Command>("my_command".into()))
                .return_once(|_| Ok(CommandResult::new(69, "")))
                .once();
        }
    );

    nash_test!(
        should_be_able_to_capture_non_zero_exit_code_of_command,
        r#"
exec `my_command` ? code;
out(code.fmt());
"#,
        "",
        |executor| {
            executor
                .expect_run()
                .with(predicate::eq::<Command>("my_command".into()))
                .return_once(|_| Ok(CommandResult::new(69, "")))
                .once();
        }
    );

    nash_test!(
        should_redirect_stdout_when_piping_commands,
        r#"
exec `command1` => `command2`;
"#,
        "",
        |executor| {
            executor
                .expect_run()
                .with(predicate::eq::<Command>(
                    Into::<Command>::into("command1")
                        .pipe("command2".into())
                        .unwrap(),
                ))
                .return_once(|_| Ok(CommandResult::new(0, "")))
                .once();
        }
    );

    nash_test!(
        should_capture_output_of_command_in_variable,
        r#"
var output = exec `command1`;
out(output);
"#,
        "",
        |executor| {
            executor
                .expect_run()
                .with(predicate::eq::<Command>("command1".into()))
                .return_once(|_| Ok(CommandResult::new(0, "hello!")))
                .once();
        }
    );
}
