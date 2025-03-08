# Async Contract Examples

## 1. Cross-Transaction Operations

```rust
#[public]
pub fn initiate_trade(ctx: &mut Context, trade_params: TradeParams) -> String {
    let params_bytes = trade_params.serialize();
    ctx.store_by_key("trade_params", &params_bytes).unwrap();
    let op_id = ctx.execute_async(&trade_params);
    op_id
}

#[public]
pub fn settle_trade(ctx: &mut Context, op_id: String) -> TradeResult {
    if !ctx.check_async_operation(&op_id) {
        return TradeResult::NotReady;
    }
    let result = ctx.get_async_result::<TradeResult>(&op_id).unwrap();
    match result {
        TradeResult::Success(trade_details) => {
            ctx.transfer_funds(trade_details.from, trade_details.to, trade_details.amount);
            result
        },
        _ => result
    }
}
```

## 2. AI/ML Use Cases

### Image Classification

```rust
#[public]
pub fn classify_image(ctx: &mut Context, image_data: Vec<u8>) -> String {
    // Start async operation for computationally intensive image classification
    let op_id = ctx.execute_async(move |_| {
        // Load the model weights from state
        let model_weights = ctx.get_state("vision_model_weights").unwrap();
        
        // Preprocess the image
        let processed_image = preprocess_image(&image_data);
        
        // Run inference (could take significant compute resources)
        let class_scores = run_image_classification(&model_weights, &processed_image);
        
        // Return top prediction
        class_scores.max_score_class()
    });
    
    op_id
}

#[public]
pub fn get_classification_result(ctx: &mut Context, op_id: String) -> Option<ClassificationResult> {
    if !ctx.check_async_operation(&op_id) {
        return None;
    }
    
    ctx.get_async_result::<ClassificationResult>(&op_id)
}
```

### Natural Language Processing

```rust
#[public]
pub fn analyze_sentiment(ctx: &mut Context, text: String) -> String {
    let op_id = ctx.execute_async(move |_| {
        // Get the NLP model parameters
        let model_params = ctx.get_state("nlp_model").unwrap();
        
        // Tokenize the input text
        let tokens = tokenize(&text);
        
        // Run the sentiment analysis model
        let sentiment_score = compute_sentiment(&model_params, &tokens);
        
        SentimentResult {
            score: sentiment_score,
            classification: if sentiment_score > 0.5 { "Positive" } else { "Negative" },
            confidence: sentiment_score.abs() * 2.0,
        }
    });
    
    op_id
}

#[public]
pub fn get_sentiment_result(ctx: &mut Context, op_id: String) -> Option<SentimentResult> {
    if !ctx.check_async_operation(&op_id) {
        return None;
    }
    
    ctx.get_async_result::<SentimentResult>(&op_id)
}
```

### Recommendation Engine

```rust
#[public]
pub fn generate_recommendations(ctx: &mut Context, user_id: String, item_count: u32) -> String {
    let op_id = ctx.execute_async(move |_| {
        // Get user history
        let user_history = ctx.get_state(&format!("user:{}:history", user_id)).unwrap_or_default();
        
        // Get recommendation model
        let model = ctx.get_state("recommendation_model").unwrap();
        
        // Get item embeddings
        let item_embeddings = ctx.get_state("item_embeddings").unwrap();
        
        // Generate user embedding from history
        let user_embedding = generate_user_embedding(&model, &user_history);
        
        // Find nearest neighbor items
        let recommended_items = find_nearest_neighbors(
            &user_embedding, 
            &item_embeddings, 
            item_count as usize
        );
        
        RecommendationResult {
            user_id,
            items: recommended_items,
            timestamp: ctx.get_timestamp(),
        }
    });
    
    op_id
}

#[public]
pub fn get_recommendations(ctx: &mut Context, op_id: String) -> Option<RecommendationResult> {
    if !ctx.check_async_operation(&op_id) {
        return None;
    }
    
    ctx.get_async_result::<RecommendationResult>(&op_id)
}
```

## 3. External System Integration

```rust
#[public]
pub fn request_price_data(ctx: &mut Context, asset_pair: String) -> String {
    let request = OracleRequest {
        asset_pair,
        min_confirmations: 3,
        timeout_blocks: 10,
    };
    let op_id = ctx.request_external_data(&request);
    ctx.store_by_key(&format!("req:{}", op_id), &request.serialize()).unwrap();
    op_id
}

#[public]
pub fn execute_with_price(ctx: &mut Context, op_id: String) -> Result<Trade, Error> {
    if !ctx.check_async_operation(&op_id) {
        return Err(Error::DataNotAvailable);
    }
    let price_data = ctx.get_async_result::<PriceData>(&op_id).ok_or(Error::InvalidData)?;
    let trade = Trade {
        price: price_data.price,
        timestamp: price_data.timestamp,
        asset_pair: price_data.asset_pair,
    };
    execute_trade(&trade)?;
    Ok(trade)
}
```

## 4. Contract-to-Contract Communication

**Producer Contract:**
```rust
#[public]
pub fn produce_async(ctx: &mut Context) -> String {
    let value: i32 = 42;
    let bytes = value.to_le_bytes().to_vec();
    match ctx.store_by_key_async(SHARED_KEY, bytes) {
        Ok(op_id) => op_id,
        Err(err) => format!("ERROR:{:?}", err),
    }
}
```

**Consumer Contract:**
```rust
#[public]
pub fn consume_async(ctx: &mut Context) -> String {
    match ctx.get_state_async(SHARED_KEY) {
        Ok(op_id) => op_id,
        Err(err) => format!("ERROR:{:?}", err),
    }
}

#[public]
pub fn get_operation_result(ctx: &mut Context, op_id: String) -> i64 {
    if !ctx.check_async_operation(&op_id) {
        return -1;
    }
    match ctx.get_async_result::<Vec<u8>>(&op_id) {
        Ok(Some(bytes)) => {
            if bytes.len() < 4 {
                return -1;
            }
            let mut array = [0u8; 4];
            array.copy_from_slice(&bytes[0..4]);
            let value = i32::from_le_bytes(array);
            value as i64
        },
        _ => -1,
    }
}
```

## 5. Parallel Execution

```rust
#[public]
pub fn process_accounts(ctx: &mut Context, accounts: Vec<Address>) -> Vec<String> {
    let mut op_ids = Vec::new();
    for account in accounts {
        let process_params = ProcessParams {
            account,
            action: "audit",
        };
        let op_id = ctx.process_async(&process_params);
        op_ids.push(op_id);
    }
    op_ids
}

#[public]
pub fn collect_results(ctx: &mut Context, op_ids: Vec<String>) -> Vec<ProcessResult> {
    let mut results = Vec::new();
    for op_id in op_ids {
        if ctx.check_async_operation(&op_id) {
            if let Ok(Some(result)) = ctx.get_async_result::<ProcessResult>(&op_id) {
                results.push(result);
            }
        }
    }
    results
}
