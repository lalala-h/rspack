use rspack_core::{context_reg_exp, ContextOptions, DependencyCategory};
use rspack_core::{BoxDependency, ConstDependency, ContextMode, ContextNameSpaceObject};
use rspack_core::{DependencyTemplate, SpanExt};
use swc_core::common::{Spanned, SyntaxContext};
use swc_core::ecma::ast::{BinExpr, CallExpr, Callee, Expr, IfStmt};
use swc_core::ecma::ast::{Lit, TryStmt, UnaryExpr, UnaryOp};
use swc_core::ecma::visit::{noop_visit_type, Visit, VisitWith};

use super::context_helper::scanner_context_module;
use super::{expr_matcher, is_unresolved_member_object_ident, is_unresolved_require};
use crate::dependency::{CommonJsRequireContextDependency, RequireHeaderDependency};
use crate::dependency::{CommonJsRequireDependency, RequireResolveDependency};
use crate::utils::{evaluate_expression, BasicEvaluatedExpression};

pub struct CommonJsImportDependencyScanner<'a> {
  dependencies: &'a mut Vec<BoxDependency>,
  presentational_dependencies: &'a mut Vec<Box<dyn DependencyTemplate>>,
  unresolved_ctxt: SyntaxContext,
  in_try: bool,
  in_if: bool,
}

impl<'a> CommonJsImportDependencyScanner<'a> {
  pub fn new(
    dependencies: &'a mut Vec<BoxDependency>,
    presentational_dependencies: &'a mut Vec<Box<dyn DependencyTemplate>>,
    unresolved_ctxt: SyntaxContext,
  ) -> Self {
    Self {
      dependencies,
      presentational_dependencies,
      unresolved_ctxt,
      in_try: false,
      in_if: false,
    }
  }

  fn add_require_resolve(&mut self, node: &CallExpr, weak: bool) {
    if !node.args.is_empty() {
      if let Some(Lit::Str(str)) = node.args.first().and_then(|x| x.expr.as_lit()) {
        self
          .dependencies
          .push(Box::new(RequireResolveDependency::new(
            node.span.real_lo(),
            node.span.real_hi(),
            str.value.to_string(),
            weak,
            node.span.into(),
            self.in_try,
          )));
      }
    }
  }

  fn replace_require_resolve(&mut self, expr: &Expr, value: &'static str) {
    if (expr_matcher::is_require(expr)
      || expr_matcher::is_require_resolve(expr)
      || expr_matcher::is_require_resolve_weak(expr))
      && is_unresolved_require(expr, self.unresolved_ctxt)
    {
      self
        .presentational_dependencies
        .push(Box::new(ConstDependency::new(
          expr.span().real_lo(),
          expr.span().real_hi(),
          value.into(),
          None,
        )));
    }
  }

  fn require_handler(&mut self, call_expr: &CallExpr) {
    if call_expr.args.len() != 1 {
      return;
    }
    let Some(ident) = call_expr.callee.as_expr().and_then(|expr| expr.as_ident()) else {
      return;
    };
    if !("require".eq(&ident.sym) && ident.span.ctxt == self.unresolved_ctxt) {
      return;
    }
    let Some(argument_expr) = call_expr.args.first().map(|arg| &arg.expr) else {
      return;
    };

    let mut process_require_item = |p: &BasicEvaluatedExpression| {
      p.is_string().then(|| {
        let dep = CommonJsRequireDependency::new(
          p.string().to_string(),
          Some(call_expr.span.into()),
          p.range().0,
          p.range().1,
          self.in_try,
        );
        self.dependencies.push(Box::new(dep));
        Some(())
      })
    };
    let param = evaluate_expression(argument_expr);
    if param.is_conditional() {
      let mut is_expression = false;
      for p in param.options() {
        if process_require_item(p).is_none() {
          is_expression = true;
        }
      }
      if !is_expression {
        self
          .presentational_dependencies
          .push(Box::new(RequireHeaderDependency::new(
            call_expr.callee.span().real_lo(),
            call_expr.callee.span().hi().0,
          )));
      }
    }

    if process_require_item(&param).is_some() {
      self
        .presentational_dependencies
        .push(Box::new(RequireHeaderDependency::new(
          call_expr.callee.span().real_lo(),
          call_expr.callee.span_hi().0,
        )));
    }
  }
}

impl Visit for CommonJsImportDependencyScanner<'_> {
  noop_visit_type!();

  fn visit_try_stmt(&mut self, node: &TryStmt) {
    self.in_try = true;
    node.visit_children_with(self);
    self.in_try = false;
  }

  fn visit_call_expr(&mut self, call_expr: &CallExpr) {
    let Callee::Expr(expr) = &call_expr.callee else {
      call_expr.visit_children_with(self);
      return;
    };

    self.require_handler(call_expr);

    if let Expr::Ident(ident) = &**expr
      && "require".eq(&ident.sym)
      && ident.span.ctxt == self.unresolved_ctxt
      && let Some(expr) = call_expr.args.first()
      && call_expr.args.len() == 1
      && expr.spread.is_none()
      && let Some((context, reg)) = scanner_context_module(expr.expr.as_ref())
    {
      // `require.resolve`
      self
        .dependencies
        .push(Box::new(CommonJsRequireContextDependency::new(
          call_expr.callee.span().real_lo(),
          call_expr.callee.span().real_hi(),
          call_expr.span.real_hi(),
          ContextOptions {
            chunk_name: None,
            mode: ContextMode::Sync,
            recursive: true,
            reg_exp: context_reg_exp(&reg, ""),
            reg_str: reg,
            include: None,
            exclude: None,
            category: DependencyCategory::CommonJS,
            request: context,
            namespace_object: ContextNameSpaceObject::Unset,
          },
          Some(call_expr.span.into()),
        )));
      return;
    }

    if is_unresolved_member_object_ident(expr, self.unresolved_ctxt) {
      if expr_matcher::is_require_resolve(expr) {
        self.add_require_resolve(call_expr, false);
        return;
      }
      if expr_matcher::is_require_resolve_weak(expr) {
        self.add_require_resolve(call_expr, true);
        return;
      }
    }
    call_expr.visit_children_with(self);
  }

  fn visit_unary_expr(&mut self, unary_expr: &UnaryExpr) {
    if let UnaryExpr {
      op: UnaryOp::TypeOf,
      arg: box expr,
      ..
    } = unary_expr
    {
      if expr_matcher::is_require(expr)
        || expr_matcher::is_require_resolve(expr)
        || expr_matcher::is_require_resolve_weak(expr)
      {
        self
          .presentational_dependencies
          .push(Box::new(ConstDependency::new(
            unary_expr.span().real_lo(),
            unary_expr.span().real_hi(),
            "'function'".into(),
            None,
          )));
      }
    }
    unary_expr.visit_children_with(self);
  }

  fn visit_if_stmt(&mut self, if_stmt: &IfStmt) {
    self.replace_require_resolve(&if_stmt.test, "true");
    self.in_if = true;
    if_stmt.visit_children_with(self);
    self.in_if = false;
  }

  fn visit_bin_expr(&mut self, bin_expr: &BinExpr) {
    let value = if self.in_if { "true" } else { "undefined" };
    self.replace_require_resolve(&bin_expr.left, value);
    self.replace_require_resolve(&bin_expr.right, value);
    bin_expr.visit_children_with(self);
  }
}