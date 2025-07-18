use std::collections::BTreeMap;

use reblessive::Stack;
use rust_decimal::Decimal;

use crate::{
	sql::{
		Array, Constant, Expression, Geometry, Id, Ident, Idiom, Number, Object, Operator, Part,
		Query, SqlValue, Statement, Statements, Strand, Thing,
	},
	syn::parser::{Parser, ParserSettings, mac::test_parse},
};

#[test]
fn parse_index_expression() {
	let value = test_parse!(parse_value_field, "a[1 + 1]").unwrap();
	let SqlValue::Idiom(x) = value else {
		panic!("not the right value type");
	};
	assert_eq!(x.0[0], Part::Field(Ident("a".to_string())));
	assert_eq!(
		x.0[1],
		Part::Value(SqlValue::Expression(Box::new(Expression::Binary {
			l: SqlValue::Number(Number::Int(1)),
			o: Operator::Add,
			r: SqlValue::Number(Number::Int(1)),
		})))
	)
}

#[test]
fn parse_coordinate() {
	let coord = test_parse!(parse_value_field, "(1.88, -18.0)").unwrap();
	let SqlValue::Geometry(Geometry::Point(x)) = coord else {
		panic!("not the right value");
	};
	assert_eq!(x.x(), 1.88);
	assert_eq!(x.y(), -18.0);
}

#[test]
fn parse_numeric_object_key() {
	let v = test_parse!(parse_value_table, "{ 00: 0 }").unwrap();
	let SqlValue::Object(object) = v else {
		panic!("not an object");
	};
	assert!(object.len() == 1);
	assert_eq!(object.get("00").cloned(), Some(SqlValue::Number(Number::Int(0))));
}

#[test]
fn parse_range_operator() {
	test_parse!(parse_value_field, "1..2").unwrap();
}

#[test]
fn parse_large_depth_object() {
	let mut text = String::new();
	let start = r#" { foo: "#;
	let middle = r#" {bar: 1} "#;
	let end = r#" } "#;

	for _ in 0..1000 {
		text.push_str(start);
	}
	text.push_str(middle);
	for _ in 0..1000 {
		text.push_str(end);
	}
	let mut parser = Parser::new_with_settings(
		text.as_bytes(),
		ParserSettings {
			query_recursion_limit: 100000,
			object_recursion_limit: 100000,
			..Default::default()
		},
	);
	let mut stack = Stack::new();
	let query = stack.enter(|stk| parser.parse_query(stk)).finish().unwrap();
	let Query(Statements(stmts)) = query;
	let Statement::Value(SqlValue::Object(ref object)) = stmts[0] else {
		panic!()
	};
	let mut object = object;
	for _ in 0..999 {
		let Some(SqlValue::Object(new_object)) = object.get("foo") else {
			panic!()
		};
		object = new_object
	}
}

#[test]
fn parse_large_depth_record_id() {
	let mut text = String::new();
	let start = r#" r"a:[ "#;
	let middle = r#" b:{c: 1} "#;
	let end = r#" ]" "#;

	for _ in 0..1000 {
		text.push_str(start);
	}
	text.push_str(middle);
	for _ in 0..1000 {
		text.push_str(end);
	}
	let mut parser = Parser::new_with_settings(
		text.as_bytes(),
		ParserSettings {
			query_recursion_limit: 100000,
			object_recursion_limit: 100000,
			..Default::default()
		},
	);
	let mut stack = Stack::new();
	let query = stack.enter(|stk| parser.parse_query(stk)).finish().unwrap();
	let Query(Statements(stmts)) = query;
	let Statement::Value(SqlValue::Thing(ref thing)) = stmts[0] else {
		panic!()
	};
	let mut thing = thing;
	for _ in 0..999 {
		let Id::Array(ref x) = thing.id else {
			panic!()
		};
		let SqlValue::Thing(ref new_thing) = x[0] else {
			panic!()
		};
		thing = new_thing
	}
}

#[test]
fn parse_recursive_record_string() {
	let res = test_parse!(parse_value_field, r#" r"a:[r"b:{c: r"d:1"}"]" "#).unwrap();
	assert_eq!(
		res,
		SqlValue::Thing(Thing {
			tb: "a".to_owned(),
			id: Id::from(Array(vec![SqlValue::Thing(Thing {
				tb: "b".to_owned(),
				id: Id::from(Object(BTreeMap::from([(
					"c".to_owned(),
					SqlValue::Thing(Thing {
						tb: "d".to_owned(),
						id: Id::from(1)
					})
				)])))
			})]))
		})
	)
}

#[test]
fn parse_record_string_2() {
	let res = test_parse!(parse_value_field, r#" r'a:["foo"]' "#).unwrap();
	assert_eq!(
		res,
		SqlValue::Thing(Thing {
			tb: "a".to_owned(),
			id: Id::from(Array(vec![SqlValue::Strand(Strand("foo".to_owned()))]))
		})
	)
}

#[test]
fn parse_i64() {
	let res = test_parse!(parse_value_field, r#" -9223372036854775808 "#).unwrap();
	assert_eq!(res, SqlValue::Number(Number::Int(i64::MIN)));

	let res = test_parse!(parse_value_field, r#" 9223372036854775807 "#).unwrap();
	assert_eq!(res, SqlValue::Number(Number::Int(i64::MAX)));
}

#[test]
fn parse_decimal() {
	let res = test_parse!(parse_value_field, r#" 0dec "#).unwrap();
	assert_eq!(res, SqlValue::Number(Number::Decimal(Decimal::ZERO)));
}

#[test]
fn constant_lowercase() {
	let out = test_parse!(parse_value_field, r#" math::pi "#).unwrap();
	assert_eq!(out, SqlValue::Constant(Constant::MathPi));

	let out = test_parse!(parse_value_field, r#" math::inf "#).unwrap();
	assert_eq!(out, SqlValue::Constant(Constant::MathInf));

	let out = test_parse!(parse_value_field, r#" math::neg_inf "#).unwrap();
	assert_eq!(out, SqlValue::Constant(Constant::MathNegInf));

	let out = test_parse!(parse_value_field, r#" time::epoch "#).unwrap();
	assert_eq!(out, SqlValue::Constant(Constant::TimeEpoch));
}

#[test]
fn constant_uppercase() {
	let out = test_parse!(parse_value_field, r#" MATH::PI "#).unwrap();
	assert_eq!(out, SqlValue::Constant(Constant::MathPi));

	let out = test_parse!(parse_value_field, r#" MATH::INF "#).unwrap();
	assert_eq!(out, SqlValue::Constant(Constant::MathInf));

	let out = test_parse!(parse_value_field, r#" MATH::NEG_INF "#).unwrap();
	assert_eq!(out, SqlValue::Constant(Constant::MathNegInf));

	let out = test_parse!(parse_value_field, r#" TIME::EPOCH "#).unwrap();
	assert_eq!(out, SqlValue::Constant(Constant::TimeEpoch));
}

#[test]
fn constant_mixedcase() {
	let out = test_parse!(parse_value_field, r#" MaTh::Pi "#).unwrap();
	assert_eq!(out, SqlValue::Constant(Constant::MathPi));

	let out = test_parse!(parse_value_field, r#" MaTh::Inf "#).unwrap();
	assert_eq!(out, SqlValue::Constant(Constant::MathInf));

	let out = test_parse!(parse_value_field, r#" MaTh::Neg_Inf "#).unwrap();
	assert_eq!(out, SqlValue::Constant(Constant::MathNegInf));

	let out = test_parse!(parse_value_field, r#" TiME::ePoCH "#).unwrap();
	assert_eq!(out, SqlValue::Constant(Constant::TimeEpoch));
}

#[test]
fn scientific_decimal() {
	let res = test_parse!(parse_value_field, r#" 9.7e-7dec "#).unwrap();
	assert!(matches!(res, SqlValue::Number(Number::Decimal(_))));
	assert_eq!(res.to_string(), "0.00000097dec")
}

#[test]
fn scientific_number() {
	let res = test_parse!(parse_value_field, r#" 9.7e-5"#).unwrap();
	assert!(matches!(res, SqlValue::Number(Number::Float(_))));
	assert_eq!(res.to_string(), "0.000097f")
}

#[test]
fn number_method() {
	let res = test_parse!(parse_value_field, r#" 9.7e-5.sin()"#).unwrap();
	let expected = SqlValue::Idiom(Idiom(vec![
		Part::Start(SqlValue::Number(Number::Float(9.7e-5))),
		Part::Method("sin".to_string(), vec![]),
	]));
	assert_eq!(res, expected);

	let res = test_parse!(parse_value_field, r#" 1.sin()"#).unwrap();
	let expected = SqlValue::Idiom(Idiom(vec![
		Part::Start(SqlValue::Number(Number::Int(1))),
		Part::Method("sin".to_string(), vec![]),
	]));
	assert_eq!(res, expected);
}

#[test]
fn datetime_error() {
	test_parse!(parse_value_field, r#" d"2001-01-01T01:01:01.9999999999" "#).unwrap_err();
}

#[test]
fn empty_string() {
	test_parse!(parse_value_field, "").unwrap_err();
}
