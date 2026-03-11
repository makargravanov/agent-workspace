use futures::future::BoxFuture;

pub async fn in_transaction<F, T, E>(pool: &sqlx::PgPool, f: F) -> Result<T, E>
where
    F: for<'c> FnOnce(&'c mut sqlx::Transaction<'_, sqlx::Postgres>) -> BoxFuture<'c, Result<T, E>>,
    E: From<sqlx::Error>,
{
    let mut tx = pool.begin().await.map_err(E::from)?;
    match f(&mut tx).await {
        Ok(value) => {
            tx.commit().await.map_err(E::from)?;
            Ok(value)
        }
        Err(e) => {
            let _ = tx.rollback().await;
            Err(e)
        }
    }
}
