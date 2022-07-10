use super::*;
use crate::ast;

pub(super) async fn andor_list<'a>(
    mut ctx: EvalContext,
    builtins: &Builtins,
    exec_sync: bool,
    show_result: &mut bool,
    list: &'a ast::AndOr<'a>,
) -> (EvalContext, u32) {
    let mut ret = 0;
    for (op, pipeline) in &list.pipelines {
        let (c, r) = exec_pipeline(ctx, builtins, exec_sync, show_result, pipeline).await;
        ctx = c;
        ret = r;
        ctx.last_return = ret;

        match op {
            ast::AndOrOp::And => {
                if ret != 0 {
                    break;
                }
            }
            ast::AndOrOp::Or => {
                if ret == 0 {
                    break;
                }
            }
        }
    }
    (ctx, ret)
}
