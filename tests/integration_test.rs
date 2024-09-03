#[cfg(test)]
mod tests {
    use core::str;
    use insta::assert_yaml_snapshot;
    use mockall::{mock, predicate};
    use nash::*;
    use serde::Serialize;
    use std::io::{self, BufReader, Cursor};

    struct MockExecutor<P: PipelineExecutor> {
        stdin: <Self as Executor>::Stdin,
        stdout: <Self as Executor>::Stdout,
        stderr: <Self as Executor>::Stderr,
        options: ExecutorOptions,
        pipeline_executor: P,
    }

    impl<P: PipelineExecutor> MockExecutor<P> {
        fn new(input: &str, pipeline_executor: P) -> Self {
            Self {
                stdin: BufReader::new(Cursor::new(
                    input.to_owned().into_bytes().into_boxed_slice(),
                )),
                stdout: Vec::new(),
                stderr: Vec::new(),
                options: ExecutorOptions::default(),
                pipeline_executor,
            }
        }
    }

    impl<P: PipelineExecutor> Executor for MockExecutor<P> {
        type Stdin = BufReader<Cursor<Box<[u8]>>>;
        type Stdout = Vec<u8>;
        type Stderr = Vec<u8>;

        fn stdin(&mut self) -> &mut Self::Stdin {
            &mut self.stdin
        }

        fn stdout(&mut self) -> &mut Self::Stdout {
            &mut self.stdout
        }

        fn stderr(&mut self) -> &mut Self::Stderr {
            &mut self.stderr
        }

        fn run_pipeline(&self, pipeline: &Pipeline) -> io::Result<PipelineOutput> {
            self.pipeline_executor.run_pipeline(pipeline)
        }

        fn options(&self) -> &ExecutorOptions {
            &self.options
        }
    }

    trait PipelineExecutor {
        fn run_pipeline(&self, pipeline: &Pipeline) -> io::Result<PipelineOutput>;
    }

    mock! {
        PipelineExecutor {}

        impl PipelineExecutor for PipelineExecutor {
            fn run_pipeline(&self, pipeline: &Pipeline) -> io::Result<PipelineOutput>;
        }
    }

    #[derive(Serialize)]
    struct CodeOutput {
        stdout: String,
        stderr: String,
        error: Option<NashError>,
        exit_code: u8,
    }

    fn run_code<F: FnOnce(&mut MockPipelineExecutor)>(
        script: &str,
        input: &str,
        setup: F,
    ) -> CodeOutput {
        let mut mock_pipeline_executor = MockPipelineExecutor::new();
        setup(&mut mock_pipeline_executor);

        let mut mock_executor = MockExecutor::new(input, mock_pipeline_executor);
        let result = nash::execute(&mut script.as_bytes(), &mut mock_executor);
        let exit_code = match &result {
            Ok(output) => output.exit_code(),
            Err(error) => error.exit_code(),
        };

        return CodeOutput {
            stdout: str::from_utf8(mock_executor.stdout()).unwrap().to_owned(),
            stderr: str::from_utf8(mock_executor.stderr()).unwrap().to_owned(),
            error: result.err(),
            exit_code,
        };
    }

    fn pipeline_success(stdout: &str, length: usize) -> PipelineOutput {
        PipelineOutput::new(Some(stdout.to_owned()), vec![0.into(); length])
    }

    macro_rules! nash_test {
        ($name:ident, $code:expr) => {
            #[test]
            fn $name() {
                assert_yaml_snapshot!(run_code($code, "", |_| {}));
            }
        };

        ($name:ident, $code:expr, $input:expr) => {
            #[test]
            fn $name() {
                assert_yaml_snapshot!(run_code($code, $input, |_| {}));
            }
        };

        ($name:ident, $code:expr, $input:expr, $setup:expr) => {
            #[test]
            fn $name() {
                assert_yaml_snapshot!(run_code($code, $input, $setup));
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
                .expect_run_pipeline()
                .with(predicate::eq(Pipeline::new(
                    vec![CommandDefinition::new(
                        "echo".to_owned(),
                        vec!["something".to_owned()],
                        false,
                    )],
                    None,
                    None,
                )))
                .return_once(|_| Ok(pipeline_success("something", 1)))
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
var mut variable = "test";
variable = 42;
"#
    );

    nash_test!(
        should_error_when_resolving_uninitialized_variable,
        r#"
var mut variable: string;
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
var mut test: void;
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
var mut index = 0;
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
var mut index = 0;
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
                .expect_run_pipeline()
                .with(predicate::eq::<Pipeline>(["my_command"].into()))
                .return_once(|_| Ok(PipelineOutput::new(None, Some(69.into()))))
                .once();
        }
    );

    nash_test!(
        should_be_able_to_capture_non_zero_exit_code_of_command,
        r#"
exec `my_command`[cap exit_code as code];
out(code.fmt());
"#,
        "",
        |executor| {
            executor
                .expect_run_pipeline()
                .with(predicate::eq::<Pipeline>(["my_command"].into()))
                .return_once(|_| Ok(PipelineOutput::new(None, Some(69.into()))))
                .once();
        }
    );

    nash_test!(
        should_be_able_to_capture_stderr_from_command,
        r#"
exec `my_command`[cap stderr as stderr];
out(stderr.fmt());
"#,
        "",
        |executor| {
            executor
                .expect_run_pipeline()
                .with(predicate::eq::<Pipeline>(Pipeline::new(
                    vec![CommandDefinition::new(
                        "my_command".to_owned(),
                        Vec::new(),
                        true,
                    )],
                    None,
                    None,
                )))
                .return_once(|_| {
                    Ok(PipelineOutput::new(
                        None,
                        Some(CommandOutput::new(
                            0,
                            Some("Something in stderr!".to_owned()),
                        )),
                    ))
                })
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
                .expect_run_pipeline()
                .with(predicate::eq::<Pipeline>(["command1", "command2"].into()))
                .return_once(|_| Ok(pipeline_success("", 2)))
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
                .expect_run_pipeline()
                .with(predicate::eq::<Pipeline>(["command1"].into()))
                .return_once(|_| Ok(pipeline_success("hello!", 1)))
                .once();
        }
    );

    nash_test!(
        should_be_able_to_get_input_from_file,
        r#"
exec open("file") => `command1`;
"#,
        "",
        |executor| {
            executor
                .expect_run_pipeline()
                .with(predicate::eq::<Pipeline>(Pipeline::new(
                    vec!["command1".into()],
                    Some(PipelineSource::File("file".to_owned())),
                    None,
                )))
                .return_once(|_| Ok(pipeline_success("", 1)))
                .once();
        }
    );

    nash_test!(
        should_be_able_to_write_input_to_file,
        r#"
exec `command1` => write("file");
"#,
        "",
        |executor| {
            executor
                .expect_run_pipeline()
                .with(predicate::eq::<Pipeline>(Pipeline::new(
                    vec!["command1".into()],
                    None,
                    Some(PipelineDestination::FileWrite("file".to_owned())),
                )))
                .return_once(|_| Ok(pipeline_success("", 1)))
                .once();
        }
    );

    nash_test!(
        should_not_be_able_to_pipe_from_write,
        r#"
exec write("test") => `command`;
"#
    );

    nash_test!(
        should_not_be_able_to_pipe_to_open,
        r#"
exec `command` => open("test");
"#
    );

    nash_test!(
        should_not_be_able_to_pipe_to_literal,
        r#"
exec `command` => "test";
"#
    );

    nash_test!(
        should_be_able_to_pipe_literal_to_command,
        r#"
var input = "input string";
exec input => `command`;
"#,
        "",
        |executor| {
            executor
                .expect_run_pipeline()
                .with(predicate::eq::<Pipeline>(Pipeline::new(
                    vec!["command".into()],
                    Some(PipelineSource::Literal("input string\n".to_owned())),
                    None,
                )))
                .return_once(|_| Ok(pipeline_success("", 1)))
                .once();
        }
    );

    nash_test!(
        should_be_able_to_append_to_a_file,
        r#"
exec "append me!" => append("file_path");
"#,
        "",
        |executor| {
            executor
                .expect_run_pipeline()
                .with(predicate::eq::<Pipeline>(Pipeline::new(
                    vec![],
                    Some(PipelineSource::Literal("append me!\n".to_owned())),
                    Some(PipelineDestination::FileAppend("file_path".to_owned())),
                )))
                .return_once(|_| Ok(pipeline_success("", 0)))
                .once();
        }
    );

    nash_test!(
        should_be_able_to_capture_data_from_commands,
        r#"
exec `command1`[
    cap exit_code, 
    cap stderr
] => `command2`[
    cap exit_code as exit_code_2, 
    cap stderr as stderr_2
];

out((exit_code, stderr, exit_code_2, stderr_2).fmt());
"#,
        "",
        |executor| {
            executor
                .expect_run_pipeline()
                .with(predicate::eq::<Pipeline>(Pipeline::new(
                    vec![
                        CommandDefinition::new("command1".to_owned(), Vec::new(), true),
                        CommandDefinition::new("command2".to_owned(), Vec::new(), true),
                    ],
                    None,
                    None,
                )))
                .return_once(|_| {
                    Ok(PipelineOutput {
                        command_outputs: vec![
                            CommandOutput::new(69, "from_command_1_stderr".to_owned().into()),
                            CommandOutput::new(70, "from_command_2_stderr".to_owned().into()),
                        ],
                        stdout: "".to_owned().into(),
                    })
                })
                .once();
        }
    );

    nash_test!(
        should_format_commands_correctly,
        r#"
    out(`my_command arg1 "arg two" "another \"one\""`.fmt());
    "#
    );

    nash_test!(
        should_format_file_handles_correctly,
        r#"        
        out(write("file.txt").fmt());
        out(open("test file").fmt());
        out(append("test \"file\"").fmt());
    "#
    );

    nash_test!(
        should_fail_to_mutate_non_mutable_variable,
        r#"
        var variable = "test";
        variable = "something else!";
        "#
    );

    nash_test!(
        should_fail_to_create_non_mutable_uninitialized_variable,
        r#"
        var variable: string;
        "#
    );

    nash_test!(
        for_loop_variable_should_be_non_immutable,
        r#"
        for index in [1] {
            index = 69;
        };
        "#
    );

    nash_test!(
        function_arguments_should_be_immutable,
        r#"
        func function(value: string) {
            value = "something else";
        }

        function("test");
        "#
    );

    nash_test!(
        captured_values_should_be_immutable,
        r#"
        exec `command`[cap stderr];
        stderr = "whatever";
        "#,
        "",
        |executor| {
            executor
                .expect_run_pipeline()
                .with(predicate::eq::<Pipeline>(Pipeline::new(
                    vec![CommandDefinition::new(
                        "command".to_owned(),
                        Vec::new(),
                        true,
                    )],
                    None,
                    None,
                )))
                .return_once(|_| {
                    Ok(PipelineOutput {
                        stdout: Some(String::new()),
                        command_outputs: vec![CommandOutput::new(
                            0,
                            Some("test_stderr".to_owned()),
                        )],
                    })
                })
                .once();
        }
    );
}
