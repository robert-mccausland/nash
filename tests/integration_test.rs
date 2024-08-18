#[cfg(test)]
mod tests {
    use insta::assert_yaml_snapshot;
    use mockall::{mock, predicate};
    use nash::*;
    use std::io::{BufRead, Read, Result, Write};

    mock! {
        pub CommandExecutor {}

        impl CommandExecutor for CommandExecutor {
            fn run(&self, command: &Command) -> Result<CommandResult>;
        }
    }

    mock! {
        pub In {}

        impl Read for In {
            fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
        }

        impl BufRead for In {
            fn fill_buf<'a>(&'a mut self) -> Result<&'a [u8]>;
            fn consume(&mut self, amt: usize);
        }
    }

    mock! {
        pub Out {}

        impl Write for Out {
            fn write(&mut self, buf: &[u8]) -> Result<usize>;
            fn flush(&mut self) -> Result<()>;
        }
    }

    fn setup_mocks<F: FnOnce(&mut MockCommandExecutor, &mut MockIn, &mut MockOut, &mut MockOut)>(
        setup: F,
    ) -> Executor {
        let mut mock_command_executor = MockCommandExecutor::new();
        let mut mock_stdin = MockIn::new();
        let mut mock_stdout = MockOut::new();
        let mut mock_stderr = MockOut::new();

        setup(
            &mut mock_command_executor,
            &mut mock_stdin,
            &mut mock_stdout,
            &mut mock_stderr,
        );

        return Executor::new(mock_command_executor, mock_stdin, mock_stdout, mock_stderr);
    }

    #[test]
    fn should_execute_valid_file() {
        let test_code = r#"
# Comments are fun!
func main() {
  var test_identifier = "Blue \"cheese\" and rice!";
  if 1 + 1 == 2 {
    out(test_identifier);
  };

  exec `echo something`;
}

main();
"#;

        let mut executor = setup_mocks(
            |mock_command_executor, _mock_stdin, mock_stdout, _mock_stderr| {
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
            },
        );

        nash::execute(&mut test_code.as_bytes(), &mut executor).unwrap();
    }

    #[test]
    fn should_fail_to_mutate_array_in_use() {
        let test_code = r#"#!/bin/nash

# Array example
var array = ["first", "second", "third"];
for value in array {
  pop(array);
};
"#;
        let mut executor =
            setup_mocks(|_mock_command_executor, _mock_stdin, _mock_stdout, _mock_stderr| {});
        assert_yaml_snapshot!(nash::execute(&mut test_code.as_bytes(), &mut executor).unwrap_err());
    }
}
