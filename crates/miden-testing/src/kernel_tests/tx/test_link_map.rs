use anyhow::Context;
use miden_objects::ONE;

use crate::TransactionContextBuilder;

#[test]
fn set() -> anyhow::Result<()> {
    let code = r#"
      use.kernel::link_map

      const.MAP_PTR=8

      begin
          push.MAP_PTR exec.link_map::new
          # => []

          # value
          push.1.2.3.4
          # key
          push.5.6.7.8
          push.MAP_PTR
          # => [map_ptr, KEY, NEW_VALUE]

          exec.link_map::set
          # => [OLD_VALUE]

          padw assert_eqw.err="old value should be the empty word"

          # value
          push.9.8.7.6
          # key
          push.5.6.7.8
          push.MAP_PTR
          # => [map_ptr, KEY, NEW_VALUE]

          exec.link_map::set
          # => [OLD_VALUE]

          push.1.2.3.4
          assert_eqw.err="old value should be the previously inserted value"
      end
    "#;

    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

    tx_context.execute_code(code).context("failed to execute code")?;

    Ok(())
}
