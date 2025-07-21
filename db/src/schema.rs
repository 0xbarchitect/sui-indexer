// @generated automatically by Diesel CLI.

diesel::table! {
    borrowers (id) {
        id -> Int4,
        #[max_length = 64]
        platform -> Varchar,
        #[max_length = 66]
        borrower -> Varchar,
        #[max_length = 66]
        obligation_id -> Nullable<Varchar>,
        status -> Int4,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    coins (id) {
        id -> Int4,
        #[max_length = 256]
        coin_type -> Varchar,
        decimals -> Int4,
        #[max_length = 256]
        name -> Nullable<Varchar>,
        #[max_length = 64]
        symbol -> Nullable<Varchar>,
        #[max_length = 32]
        price_pyth -> Nullable<Varchar>,
        #[max_length = 32]
        price_supra -> Nullable<Varchar>,
        #[max_length = 32]
        price_switchboard -> Nullable<Varchar>,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        #[max_length = 256]
        pyth_feed_id -> Nullable<Varchar>,
        #[max_length = 256]
        pyth_info_object_id -> Nullable<Varchar>,
        pyth_latest_updated_at -> Nullable<Timestamp>,
        #[max_length = 32]
        pyth_ema_price -> Nullable<Varchar>,
        pyth_decimals -> Nullable<Int4>,
        navi_asset_id -> Nullable<Int4>,
        navi_oracle_id -> Nullable<Int4>,
        #[max_length = 66]
        navi_feed_id -> Nullable<Varchar>,
        #[max_length = 32]
        hermes_price -> Nullable<Varchar>,
        hermes_latest_updated_at -> Nullable<Timestamp>,
        vaa -> Nullable<Text>,
    }
}

diesel::table! {
    metrics (id) {
        id -> Int4,
        latest_seq_number -> Int4,
        total_checkpoints -> Int4,
        total_processed_checkpoints -> Int4,
        max_processing_time -> Float4,
        min_processing_time -> Float4,
        avg_processing_time -> Float4,
        max_lagging -> Float4,
        min_lagging -> Float4,
        avg_lagging -> Float4,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    pool_ticks (id) {
        id -> Int4,
        #[max_length = 66]
        address -> Varchar,
        tick_index -> Int4,
        #[max_length = 64]
        liquidity_net -> Nullable<Varchar>,
        #[max_length = 64]
        liquidity_gross -> Nullable<Varchar>,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    pools (id) {
        id -> Int4,
        #[max_length = 64]
        exchange -> Varchar,
        #[max_length = 66]
        address -> Varchar,
        #[max_length = 64]
        liquidity -> Nullable<Varchar>,
        #[max_length = 32]
        current_sqrt_price -> Nullable<Varchar>,
        tick_spacing -> Nullable<Int4>,
        fee_rate -> Nullable<Int4>,
        is_pause -> Nullable<Bool>,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        coins -> Text,
        #[max_length = 256]
        coin_amounts -> Nullable<Varchar>,
        #[max_length = 256]
        weights -> Nullable<Varchar>,
        #[max_length = 256]
        fees_swap_in -> Nullable<Varchar>,
        #[max_length = 256]
        fees_swap_out -> Nullable<Varchar>,
        current_tick_index -> Nullable<Int4>,
        #[max_length = 256]
        pool_type -> Nullable<Varchar>,
    }
}

diesel::table! {
    shared_objects (id) {
        id -> Int4,
        #[max_length = 66]
        object_id -> Varchar,
        initial_shared_version -> Int8,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    user_borrows (id) {
        id -> Int4,
        #[max_length = 64]
        platform -> Varchar,
        #[max_length = 66]
        borrower -> Varchar,
        #[max_length = 256]
        coin_type -> Varchar,
        #[max_length = 64]
        amount -> Varchar,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        #[max_length = 256]
        obligation_id -> Nullable<Varchar>,
        #[max_length = 256]
        debt_borrow_index -> Nullable<Varchar>,
    }
}

diesel::table! {
    user_deposits (id) {
        id -> Int4,
        #[max_length = 64]
        platform -> Varchar,
        #[max_length = 66]
        borrower -> Varchar,
        #[max_length = 256]
        coin_type -> Varchar,
        #[max_length = 64]
        amount -> Varchar,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        #[max_length = 256]
        obligation_id -> Nullable<Varchar>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    borrowers,
    coins,
    metrics,
    pool_ticks,
    pools,
    shared_objects,
    user_borrows,
    user_deposits,
);
