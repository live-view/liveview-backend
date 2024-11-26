use alloy::sol;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    ERC20,
    "abi/ERC20.json",
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    ERC721,
    "abi/ERC721.json",
);
