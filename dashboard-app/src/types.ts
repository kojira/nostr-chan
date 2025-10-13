export interface BotData {
  pubkey: string;
  secretkey: string;
  prompt: string;
  content: string;
  status: number; // 0: active, 1: inactive
}

export interface BotRequest {
  secretkey: string;
  prompt: string;
  content: string;
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

