//pub struct Redirect<'a> {
//    command: &'a Command<'a>,
//    operator: RedirectionType,
//    fname: Arg<'a>,
//    fd: int,
//    dupfd: int,
//}

//pub enum RedirectionType {
//    TO,     // fd > fname
//    CLOBBER,// fd >| fname
//    FROM,   // fd < fname
//    FROMTO, // fd <> fname
//    APPEND, // fd >> fname
//    TOFD,   // fd <& dupfd
//    FROMFD, // fd >& dupfd
//}
