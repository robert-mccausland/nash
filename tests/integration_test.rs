#[cfg(test)]
mod tests {
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

    #[test]
    fn should_execute_valid_file() {
        let test_file = r#"
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

        let mut executor =
            Executor::new(mock_command_executor, mock_stdin, mock_stdout, mock_stderr);

        nash::execute(&mut test_file.as_bytes(), &mut executor).unwrap();
    }
}
