export interface BotData {
  pubkey: string;
  secretkey: string;
  prompt: string;
  content: string;
  status: number; // 0: active, 1: inactive
  air_reply_single_ratio?: number;
}

export interface BotRequest {
  secretkey: string;
  prompt: string;
  content: string;
  air_reply_single_ratio?: number;
}

export interface Stats {
  bot_status: BotStatus;
  reply_stats: ReplyStats;
  conversation_stats: ConversationStats;
  rag_stats: RagStats;
  error_log: ErrorEntry[];
}

export interface BotStatus {
  online: boolean;
  uptime_seconds: number;
  last_reply_timestamp: number;
  connected_relays: string[];
}

export interface ReplyStats {
  today: number;
  this_week: number;
  this_month: number;
  total: number;
}

export interface ConversationStats {
  unique_users: number;
  rate_limited_users: number;
}

export interface RagStats {
  vectorized_events: number;
  total_events: number;
  pending_vectorization: number;
  total_searches: number;
  average_similarity: number;
}

export interface ErrorEntry {
  timestamp: number;
  error_type: string;
  message: string;
}

export interface VectorizedEvent {
  id: number;
  event_id: string;
  pubkey: string;
  kind: number;
  content: string;
  created_at: number;
  received_at: number;
  kind0_name: string | null;
  is_japanese: boolean;
  has_embedding: boolean;
  event_type: string | null;
  event_json?: string; // JSONとして取得する場合
}

export interface EventsResponse {
  events: VectorizedEvent[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

