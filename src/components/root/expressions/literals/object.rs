use std::{cell::RefCell, collections::HashMap, rc::Rc};

use serde::Serialize;

use crate::{
    components::{
        root::{
            expressions::{Expression, ExpressionComponent},
            identifier::Identifier,
        },
        stack::Stack,
        values::{ObjectField, ObjectFieldType, Type, Value},
        EvaluationException, EvaluationResult, PostProcessContext, Tokens,
    },
    constants::MUT,
    errors::PostProcessError,
    lexer::TokenValue,
    Executor,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ObjectLiteral {
    fields: HashMap<String, (bool, Expression)>,
}

impl ExpressionComponent for ObjectLiteral {
    fn try_parse<'a, I: Iterator<Item = &'a crate::lexer::Token<'a>>>(
        tokens: &mut crate::utils::iterators::Backtrackable<I>,
    ) -> Result<Option<Self>, crate::ParserError>
    where
        Self: Sized,
    {
        let checkpoint = tokens.checkpoint();

        let Some(TokenValue::LeftCurly()) = tokens.peek_value() else {
            return Ok(None);
        };
        tokens.next();

        let mut is_first_iteration = true;
        let mut fields = HashMap::new();
        loop {
            if let Some(TokenValue::RightCurly()) = tokens.peek_value() {
                tokens.next();
                break;
            }

            let mutable = if let Some(TokenValue::Keyword(MUT)) = tokens.peek_value() {
                tokens.next();
                true
            } else {
                false
            };

            let Some(field_name) = Identifier::try_parse(tokens)? else {
                // We could be in a block instead of an object in these cases
                if is_first_iteration && !mutable {
                    tokens.backtrack(checkpoint);
                    return Ok(None);
                }
                return Err("Expected identifier".into());
            };

            let Some(TokenValue::Colon()) = tokens.peek_value() else {
                // We could be in a block instead of an object in these cases
                if is_first_iteration && !mutable {
                    tokens.backtrack(checkpoint);
                    return Ok(None);
                }
                return Err("Expected : after field name".into());
            };
            tokens.next();

            let field_value = Expression::parse(tokens)?;

            fields.insert(field_name.value.to_owned(), (mutable, field_value));

            let Some(TokenValue::Comma()) = tokens.peek_value() else {
                if let Some(TokenValue::RightCurly()) = tokens.peek_value() {
                    continue;
                }

                return Err("Expected , or } after field value".into());
            };
            tokens.next();
            is_first_iteration = false;
        }

        return Ok(Some(Self { fields }));
    }

    fn get_type(&self, context: &mut PostProcessContext) -> Result<Type, PostProcessError> {
        Ok(Type::Object(
            self.fields
                .iter()
                .map(|(key, (mutable, value))| {
                    Ok::<_, PostProcessError>((
                        key.clone(),
                        ObjectFieldType {
                            mutable: *mutable,
                            value: value.get_type(context)?,
                        },
                    ))
                })
                .collect::<Result<HashMap<_, _>, _>>()?,
        ))
    }

    fn evaluate<E: Executor>(
        &self,
        stack: &mut Stack,
        executor: &mut E,
    ) -> EvaluationResult<Value> {
        let value = self
            .fields
            .iter()
            .map(|(key, (mutable, value))| {
                Ok::<_, EvaluationException>((
                    key.clone(),
                    ObjectField {
                        mutable: *mutable,
                        value: value.evaluate(stack, executor)?,
                    },
                ))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(Value::Object(Rc::new(RefCell::new(value))))
    }
}
