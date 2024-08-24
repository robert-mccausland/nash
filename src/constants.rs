macro_rules! define_keywords {
    ($($name:ident => $value:expr),*) => {
        $(
            pub const $name: &str = $value;
        )*

        pub const KEYWORDS: [&str; count!($($name,)*)] = [
            $($name),*
        ];
    }
}

macro_rules! count {
    ($first:tt, $($rest:tt, )*) => (1usize + count!($($rest,)*));
    () => (0usize);
}

// Not a keyword, but it is a special identifier
pub const UNDERSCORE: &str = "_";

define_keywords!(
    IF => "if",
    ELSE => "else",
    FUNC => "func",
    VAR => "var",
    EXEC => "exec",
    TRUE => "true",
    FALSE => "false",
    FOR => "for",
    IN => "in",
    WHILE => "while",
    RETURN => "return",
    BREAK => "break",
    CONTINUE => "continue",
    EXIT => "exit"
);
