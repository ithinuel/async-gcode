use crate::stream::PushBackable;
use core::marker;
use futures::{Stream, StreamExt};

pub(crate) async fn skip_whitespaces<S>(input: &mut S) -> Option<()>
where
    S: Stream<Item = u8> + marker::Unpin + PushBackable<Item = <S as Stream>::Item>,
{
    let b = input
        .filter(|c| futures::future::ready(*c != b' '))
        .next()
        .await?;
    input.push_back(b);
    Some(())
}
