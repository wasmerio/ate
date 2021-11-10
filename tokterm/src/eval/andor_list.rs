use crate::ast;
use super::*;

pub(super) async fn andor_list<'a>(
    ctx: &mut EvalContext,
    builtins: &Builtins,
    exec_sync: bool,
    show_result: &mut bool,
    list: &'a ast::AndOr<'a>
) -> i32
{
    let mut ret = 0;
    for (op, pipeline) in &list.pipelines
    {
        ret = exec_pipeline(ctx, builtins, exec_sync, show_result, pipeline).await;
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
    ret
}