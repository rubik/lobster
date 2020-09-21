use custom_error::custom_error;

custom_error! { pub ExecutionError
    OrderNotFound = "could not find the specified order",
    RemoveFailed = "failed to remove order",
}
