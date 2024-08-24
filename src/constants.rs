macro_rules! define_keywords {
    ($($name:ident => $value:expr),*) => {
        pub const KEYWORDS: [&str; count_identifiers!($($name),*)] = [
            $($value),*
        ];

        $(
            pub const $name: &str = $value;
        )*
    }
}

macro_rules! count_identifiers {
    ($($identifiers:ident),*) => {
        <[()]>::len(&[$(count_identifiers!(@replace $identifiers)),*])
    };
    (@replace $identifiers:ident) => { () };
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
