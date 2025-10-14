import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  Container, Box, Typography, IconButton, Paper, List, ListItem, ListItemText,
  Chip, CircularProgress, Button
} from '@mui/material';
import { ArrowBack, ChatBubble, Person, AccessTime } from '@mui/icons-material';
import { useBots } from '../hooks/useBots';

interface BotReply {
  event_id: string;
  content: string;
  created_at: number;
  reply_to_event_id?: string;
  reply_to_user?: string;
}

export const BotDetailPage = () => {
  const { pubkey } = useParams<{ pubkey: string }>();
  const navigate = useNavigate();
  const { bots } = useBots();
  const [replies, setReplies] = useState<BotReply[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [hasMore, setHasMore] = useState(true);

  const bot = bots.find(b => b.pubkey === pubkey);

  useEffect(() => {
    if (pubkey) {
      loadReplies();
    }
  }, [pubkey]);

  const loadReplies = async (offset = 0) => {
    try {
      if (offset === 0) {
        setLoading(true);
      } else {
        setLoadingMore(true);
      }

      const response = await fetch(`/api/bots/${pubkey}/replies?limit=50&offset=${offset}`);
      if (response.ok) {
        const data: BotReply[] = await response.json();
        if (offset === 0) {
          setReplies(data);
        } else {
          setReplies(prev => [...prev, ...data]);
        }
        setHasMore(data.length === 50);
      }
    } catch (error) {
      console.error('返信履歴の取得エラー:', error);
    } finally {
      setLoading(false);
      setLoadingMore(false);
    }
  };

  const formatDate = (timestamp: number) => {
    const date = new Date(timestamp * 1000);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return 'たった今';
    if (diffMins < 60) return `${diffMins}分前`;
    if (diffHours < 24) return `${diffHours}時間前`;
    if (diffDays < 7) return `${diffDays}日前`;
    
    return date.toLocaleDateString('ja-JP', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    });
  };

  const getNoteLink = (eventId: string) => {
    try {
      // nostr-tools を使ってnote1形式に変換する場合はここで実装
      // 簡易版: event IDをそのまま表示
      return `nostr:note1${eventId.substring(0, 8)}...`;
    } catch {
      return eventId;
    }
  };

  if (loading) {
    return (
      <Container maxWidth="lg" sx={{ py: 4, display: 'flex', justifyContent: 'center' }}>
        <CircularProgress />
      </Container>
    );
  }

  if (!bot) {
    return (
      <Container maxWidth="lg" sx={{ py: 4 }}>
        <Typography>Botが見つかりません</Typography>
      </Container>
    );
  }

  return (
    <Container maxWidth="lg" sx={{ py: 4 }}>
      {/* ヘッダー */}
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mb: 4 }}>
        <IconButton onClick={() => navigate('/bots')} size="large">
          <ArrowBack />
        </IconButton>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, flex: 1 }}>
          <ChatBubble sx={{ fontSize: 32, color: 'primary.main' }} />
          <Box>
            <Typography variant="h4" fontWeight="bold">
              {bot.content ? JSON.parse(bot.content).name || 'Bot' : 'Bot'} の返信履歴
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ fontFamily: 'monospace' }}>
              {pubkey?.substring(0, 16)}...
            </Typography>
          </Box>
        </Box>
        <Chip label={`${replies.length}件の返信`} color="primary" />
      </Box>

      {/* 返信一覧 */}
      {replies.length === 0 ? (
        <Paper sx={{ p: 4, textAlign: 'center' }}>
          <Typography color="text.secondary">まだ返信がありません</Typography>
        </Paper>
      ) : (
        <>
          <List>
            {replies.map((reply) => (
              <Paper key={reply.event_id} sx={{ mb: 2, p: 2 }}>
                <ListItem sx={{ flexDirection: 'column', alignItems: 'flex-start', p: 0 }}>
                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1, width: '100%' }}>
                    <AccessTime sx={{ fontSize: 16, color: 'text.secondary' }} />
                    <Typography variant="caption" color="text.secondary">
                      {formatDate(reply.created_at)}
                    </Typography>
                    {reply.reply_to_user && (
                      <>
                        <Person sx={{ fontSize: 16, color: 'text.secondary', ml: 2 }} />
                        <Typography variant="caption" color="text.secondary" sx={{ fontFamily: 'monospace' }}>
                          {reply.reply_to_user.substring(0, 8)}...へ返信
                        </Typography>
                      </>
                    )}
                  </Box>
                  <ListItemText
                    primary={reply.content}
                    primaryTypographyProps={{
                      sx: { whiteSpace: 'pre-wrap', wordBreak: 'break-word' }
                    }}
                  />
                  <Box sx={{ mt: 1 }}>
                    <Typography variant="caption" color="text.secondary" sx={{ fontFamily: 'monospace' }}>
                      {getNoteLink(reply.event_id)}
                    </Typography>
                  </Box>
                </ListItem>
              </Paper>
            ))}
          </List>

          {hasMore && (
            <Box sx={{ display: 'flex', justifyContent: 'center', mt: 3 }}>
              <Button
                variant="outlined"
                onClick={() => loadReplies(replies.length)}
                disabled={loadingMore}
              >
                {loadingMore ? <CircularProgress size={24} /> : 'さらに読み込む'}
              </Button>
            </Box>
          )}
        </>
      )}
    </Container>
  );
};

