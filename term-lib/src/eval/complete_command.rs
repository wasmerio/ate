use super::*;
use crate::ast;

pub(super) async fn complete_command<'a>(
    mut ctx: EvalContext,
    builtins: &Builtins,
    cc: &'a ast::CompleteCommand<'a>,
    show_result: &mut bool,
) -> (EvalContext, u32) {
    let mut ret = 0;
    for (op, list) in &cc.and_ors {
        let (c, r) = andor_list(ctx, builtins, *op != ast::TermOp::Amp, show_result, list).await;
        ctx = c;
        ret = r;
    }
    (ctx, ret)
}
