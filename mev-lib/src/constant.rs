// dexes
pub const CETUS_SWAP_EVENT: &str =
    "0x1eabed72c53feb3805120a081dc15963c204dc8d091542592abaf7a35689b2fb::pool::SwapEvent";

pub const CETUS_ADD_LIQUIDITY_EVENT: &str =
    "0x1eabed72c53feb3805120a081dc15963c204dc8d091542592abaf7a35689b2fb::pool::AddLiquidityEvent";

pub const CETUS_REMOVE_LIQUIDITY_EVENT: &str = "0x1eabed72c53feb3805120a081dc15963c204dc8d091542592abaf7a35689b2fb::pool::RemoveLiquidityEvent";

pub const BLUEFIN_SWAP_EVENT: &str =
    "0x3492c874c1e3b3e2984e8c41b589e642d4d0a5d6459e5a9cfc2d52fd7c89c267::events::AssetSwap";

pub const BLUEFIN_TICK_UPDATED_EVENT: &str =
    "0xf1962ddb76a7f9968b4e597278d3cc717a00620cc421b00e3429c5c071eba26a::events::PoolTickUpdate";

pub const TURBOS_SWAP_EVENT: &str =
    "0x91bfbc386a41afcfd9b2533058d7e915a1d3829089cc268ff4333d54d6339ca1::pool::SwapEvent";

pub const TURBOS_ADD_LIQUIDITY_EVENT: &str =
    "0x91bfbc386a41afcfd9b2533058d7e915a1d3829089cc268ff4333d54d6339ca1::pool::MintEvent";

pub const TURBOS_REMOVE_LIQUIDITY_EVENT: &str =
    "0x91bfbc386a41afcfd9b2533058d7e915a1d3829089cc268ff4333d54d6339ca1::pool::BurnEvent";

pub const MOMENTUM_SWAP_EVENT: &str =
    "0x70285592c97965e811e0c6f98dccc3a9c2b4ad854b3594faab9597ada267b860::trade::SwapEvent";

pub const MOMENTUM_ADD_LIQUIDITY_EVENT: &str =
    "0x70285592c97965e811e0c6f98dccc3a9c2b4ad854b3594faab9597ada267b860::liquidity::AddLiquidityEvent";

pub const MOMENTUM_REMOVE_LIQUIDITY_EVENT: &str = 
    "0x70285592c97965e811e0c6f98dccc3a9c2b4ad854b3594faab9597ada267b860::liquidity::RemoveLiquidityEvent";

pub const FLOWX_SWAP_EVENT: &str =
    "0x25929e7f29e0a30eb4e692952ba1b5b65a3a4d65ab5f2a32e1ba3edcb587f26d::pool::Swap";

pub const FLOWX_MODIFY_LIQUIDITY_EVENT: &str =
    "0x25929e7f29e0a30eb4e692952ba1b5b65a3a4d65ab5f2a32e1ba3edcb587f26d::pool::ModifyLiquidity";

pub const BLUEMOVE_SWAP_EVENT: &str =
    "0xb24b6789e088b876afabca733bed2299fbc9e2d6369be4d1acfa17d8145454d9::swap::Swap_Event";

pub const AFTERMATH_SWAP_EVENT: &str =
    "0xc4049b2d1cc0f6e017fda8260e4377cecd236bd7f56a54fee120816e72e2e0dd::events::SwapEventV2";

pub const OBRIC_SWAP_EVENT: &str =
    "0x200e762fa2c49f3dc150813038fbf22fd4f894ac6f23ebe1085c62f2ef97f1ca::obric::ObricSwapEvent";

// navi events
pub const NAVI_BORROW_EVENT: &str =
    "0xd899cf7d2b5db716bd2cf55599fb0d5ee38a3061e7b6bb6eebf73fa5bc4c81ca::lending::BorrowEvent";
pub const NAVI_DEPOSIT_EVENT: &str =
    "0xd899cf7d2b5db716bd2cf55599fb0d5ee38a3061e7b6bb6eebf73fa5bc4c81ca::lending::DepositEvent";
pub const NAVI_REPAY_EVENT: &str =
    "0xd899cf7d2b5db716bd2cf55599fb0d5ee38a3061e7b6bb6eebf73fa5bc4c81ca::lending::RepayEvent";
pub const NAVI_WITHDRAW_EVENT: &str =
    "0xd899cf7d2b5db716bd2cf55599fb0d5ee38a3061e7b6bb6eebf73fa5bc4c81ca::lending::WithdrawEvent";
pub const NAVI_LIQUIDATE_EVENT: &str =
    "0xc6374c7da60746002bfee93014aeb607e023b2d6b25c9e55a152b826dbc8c1ce::lending::LiquidationEvent";
pub const NAVI_STATE_UPDATED_EVENT: &str = "0x834a86970ae93a73faf4fff16ae40bdb72b91c47be585fff19a2af60a19ddca3::logic::StateUpdated";

// suilend events
pub const SUILEND_BORROW_EVENT: &str = "0xf95b06141ed4a174f239417323bde3f209b972f5930d8521ea38a52aff3a6ddf::lending_market::BorrowEvent";
pub const SUILEND_DEPOSIT_EVENT: &str = "0xf95b06141ed4a174f239417323bde3f209b972f5930d8521ea38a52aff3a6ddf::lending_market::DepositEvent";
pub const SUILEND_REPAY_EVENT: &str = "0xf95b06141ed4a174f239417323bde3f209b972f5930d8521ea38a52aff3a6ddf::lending_market::RepayEvent";
pub const SUILEND_WITHDRAW_EVENT: &str = "0xf95b06141ed4a174f239417323bde3f209b972f5930d8521ea38a52aff3a6ddf::lending_market::WithdrawEvent";
pub const SUILEND_LIQUIDATE_EVENT: &str = "0xf95b06141ed4a174f239417323bde3f209b972f5930d8521ea38a52aff3a6ddf::lending_market::LiquidateEvent";


// scallop events
pub const SCALLOP_BORROW_EVENT: &str = "0xefe8b36d5b2e43728cc323298626b83177803521d195cfb11e15b910e892fddf::borrow::BorrowEvent";
pub const SCALLOP_BORROW_EVENT_V2: &str =
    "0xefe8b36d5b2e43728cc323298626b83177803521d195cfb11e15b910e892fddf::borrow::BorrowEventV2";
pub const SCALLOP_BORROW_EVENT_V3: &str =
    "0x6e641f0dca8aedab3101d047e96439178f16301bf0b57fe8745086ff1195eb3e::borrow::BorrowEventV3";

pub const SCALLOP_DEPOSIT_EVENT: &str = "0xefe8b36d5b2e43728cc323298626b83177803521d195cfb11e15b910e892fddf::deposit_collateral::CollateralDepositEvent";
pub const SCALLOP_REPAY_EVENT: &str =
    "0xefe8b36d5b2e43728cc323298626b83177803521d195cfb11e15b910e892fddf::repay::RepayEvent";
pub const SCALLOP_WITHDRAW_EVENT: &str = "0xefe8b36d5b2e43728cc323298626b83177803521d195cfb11e15b910e892fddf::withdraw_collateral::CollateralWithdrawEvent";

pub const SCALLOP_LIQUIDATE_EVENT_V2: &str =
    "0x6e641f0dca8aedab3101d047e96439178f16301bf0b57fe8745086ff1195eb3e::liquidate::LiquidateEventV2";

// oracles
pub const PYTH_UPDATE_PRICE_EVENT: &str = "0x8d97f1cd6ac663735be08d1d2b6d02a159e711586461306ce60a2b7a6a565a9e::event::PriceFeedUpdateEvent";

// coin types
pub const SUI_COIN: &str = "0x2::sui::SUI";
pub const SUI_DECIMALS : usize = 9;

pub const USDC_COIN: &str =
    "0xdba34672e30cb065b1f93e3ab55318768fd6fef66c15942c9f7cb846e2f900e7::usdc::USDC";
pub const USDC_DECIMALS: usize = 6;

pub const CLOCK_OBJECT_ID: &str = "0x6";

// exchanges names
pub const CETUS_EXCHANGE: &str = "cetus";
pub const BLUEFIN_EXCHANGE: &str = "bluefin";
pub const TURBOS_EXCHANGE: &str = "turbos";
pub const AFTERMATH_EXCHANGE: &str = "aftermath";
pub const MOMENTUM_EXCHANGE: &str = "momentum";
pub const FLOWX_EXCHANGE: &str = "flowx";
pub const BLUEMOVE_EXCHANGE: &str = "bluemove";
pub const OBRIC_EXCHANGE: &str = "obric";

// lending names
pub const NAVI_LENDING: &str = "navi";
pub const SCALLOP_LENDING: &str = "scallop";
pub const SUILEND_LENDING: &str = "suilend";

// oracles names
pub const PYTH_ORACLE: &str = "pyth";

// pyth
pub const PYTH_PRICE_UPDATE_MESSAGE_TYPE: &str = "price_update";

// liquidator

pub const PENDING_STATUS: i32 = 0;
pub const READY_STATUS: i32 = 1;
pub const PROCESSING_STATUS: i32 = 1;
pub const SUCCEED_STATUS: i32 = 2;
pub const FAILED_STATUS: i32 = -1;
pub const ABNORMAL_STATUS: i32 = -2;
