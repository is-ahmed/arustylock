

#[derive(Serialize, Deserialize, Clone)]
struct Password {
    id: usize,
    username: String,
    password: String,
    created_at: DateTime<Utc>
}