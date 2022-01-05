use super::*;
use crate::ast;

pub(super) async fn complete_command<'a>(
    ctx: &mut EvalContext,
    builtins: &Builtins,
    cc: &'a ast::CompleteCommand<'a>,
    show_result: &mut bool,
) -> u32 {
    let mut ret = 0;
    for (op, list) in &cc.and_ors {
        ret = andor_list(ctx, builtins, *op != ast::TermOp::Amp, show_result, list).await;
        ctx.last_return = ret;
    }
    ret
}
