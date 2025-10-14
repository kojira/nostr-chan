import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  Container, Box, Typography, IconButton, Paper, List, ListItem, ListItemText,
  Chip, CircularProgress, Button, TextField, Select, MenuItem, FormControl,
  InputLabel, TablePagination, Dialog, DialogTitle, DialogContent, DialogActions
} from '@mui/material';
import { ArrowBack, ChatBubble, Person, AccessTime, Code } from '@mui/icons-material';
import { useBots } from '../hooks/useBots';

interface BotReply {
  event_id: string;
  content: string;
  created_at: number;
  reply_to_event_id?: string;
  reply_to_user?: string;
  event_json: string;
}

export const BotDetailPage = () => {
  const { pubkey } = useParams<{ pubkey: string }>();
  const navigate = useNavigate();
  const { bots } = useBots();
  const [replies, setReplies] = useState<BotReply[]>([]);
  const [loading, setLoading] = useState(true);
  const [initialLoading, setInitialLoading] = useState(true);
  const [page, setPage] = useState(0);
  const [rowsPerPage, setRowsPerPage] = useState(25);
  const [searchText, setSearchText] = useState('');
  const [actualSearchText, setActualSearchText] = useState('');
  const [isComposing, setIsComposing] = useState(false);
  const [sortBy, setSortBy] = useState<'created_at' | 'content'>('created_at');
  const [sortOrder, setSortOrder] = useState<'ASC' | 'DESC'>('DESC');
  const [jsonDialogOpen, setJsonDialogOpen] = useState(false);
  const [selectedJson, setSelectedJson] = useState<string>('');

  const bot = bots.find(b => b.pubkey === pubkey);

  useEffect(() => {
    if (pubkey) {
      loadReplies();
    }
  }, [pubkey, page, rowsPerPage, actualSearchText, sortBy, sortOrder]);

  const loadReplies = async () => {
    try {
      setLoading(true);
      const offset = page * rowsPerPage;
      const params = new URLSearchParams({
        limit: rowsPerPage.toString(),
        offset: offset.toString(),
        sort_by: sortBy,
        sort_order: sortOrder,
      });
      
      if (actualSearchText) {
        params.append('search', actualSearchText);
      }

      const response = await fetch(`/api/bots/${pubkey}/replies?${params}`);
      if (response.ok) {
        const data: BotReply[] = await response.json();
        setReplies(data);
      }
    } catch (error) {
      console.error('返信履歴の取得エラー:', error);
    } finally {
      setLoading(false);
      setInitialLoading(false);
    }
  };

  const formatDate = (timestamp: number) => {
    const date = new Date(timestamp * 1000);
    return date.toLocaleString('ja-JP', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  };

  const getNoteLink = (eventId: string) => {
    return `nostr:note1${eventId.substring(0, 8)}...`;
  };

  const handleSearchChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const value = event.target.value;
    setSearchText(value);
    // 日本語入力中でなければ即座に検索実行
    if (!isComposing) {
      setActualSearchText(value);
      setPage(0);
    }
  };

  const handleCompositionStart = () => {
    setIsComposing(true);
  };

  const handleCompositionEnd = (event: React.CompositionEvent<HTMLInputElement>) => {
    setIsComposing(false);
    const value = (event.target as HTMLInputElement).value;
    setActualSearchText(value);
    setPage(0);
  };

  const handleChangePage = (_event: unknown, newPage: number) => {
    setPage(newPage);
  };

  const handleChangeRowsPerPage = (event: React.ChangeEvent<HTMLInputElement>) => {
    setRowsPerPage(parseInt(event.target.value, 10));
    setPage(0);
  };

  const handleOpenJson = (jsonString: string) => {
    try {
      const formatted = JSON.stringify(JSON.parse(jsonString), null, 2);
      setSelectedJson(formatted);
      setJsonDialogOpen(true);
    } catch {
      setSelectedJson(jsonString);
      setJsonDialogOpen(true);
    }
  };

  if (initialLoading) {
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
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mb: 3 }}>
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
      </Box>

      {/* フィルタとソート */}
      <Paper sx={{ p: 2, mb: 2 }}>
        <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap', alignItems: 'center' }}>
          <TextField
            label="検索"
            variant="outlined"
            size="small"
            value={searchText}
            onChange={handleSearchChange}
            onCompositionStart={handleCompositionStart}
            onCompositionEnd={handleCompositionEnd}
            sx={{ minWidth: 200, flex: 1 }}
            placeholder="本文で検索..."
          />
          <FormControl size="small" sx={{ minWidth: 150 }}>
            <InputLabel>ソート</InputLabel>
            <Select
              value={sortBy}
              label="ソート"
              onChange={(e) => {
                setSortBy(e.target.value as 'created_at' | 'content');
                setPage(0);
              }}
            >
              <MenuItem value="created_at">投稿日時</MenuItem>
              <MenuItem value="content">本文</MenuItem>
            </Select>
          </FormControl>
          <FormControl size="small" sx={{ minWidth: 120 }}>
            <InputLabel>順序</InputLabel>
            <Select
              value={sortOrder}
              label="順序"
              onChange={(e) => {
                setSortOrder(e.target.value as 'ASC' | 'DESC');
                setPage(0);
              }}
            >
              <MenuItem value="DESC">降順</MenuItem>
              <MenuItem value="ASC">昇順</MenuItem>
            </Select>
          </FormControl>
        </Box>
      </Paper>

      {/* 返信一覧 */}
      {loading ? (
        <Box sx={{ display: 'flex', justifyContent: 'center', py: 4 }}>
          <CircularProgress size={40} />
        </Box>
      ) : replies.length === 0 ? (
        <Paper sx={{ p: 4, textAlign: 'center' }}>
          <Typography color="text.secondary">
            {actualSearchText ? '検索結果が見つかりません' : 'まだ返信がありません'}
          </Typography>
        </Paper>
      ) : (
        <>
          <List>
            {replies.map((reply) => (
              <Paper 
                key={reply.event_id} 
                sx={{ 
                  mb: 2, 
                  p: 2,
                  cursor: 'pointer',
                  transition: 'all 0.2s',
                  '&:hover': {
                    bgcolor: 'action.hover',
                    boxShadow: 2,
                  }
                }}
                onClick={() => handleOpenJson(reply.event_json)}
              >
                <ListItem sx={{ flexDirection: 'column', alignItems: 'flex-start', p: 0 }}>
                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1, width: '100%' }}>
                    <AccessTime sx={{ fontSize: 16, color: 'text.secondary' }} />
                    <Typography variant="caption" color="text.secondary" sx={{ fontFamily: 'monospace' }}>
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
                    <Box sx={{ ml: 'auto' }}>
                      <Chip 
                        icon={<Code sx={{ fontSize: 14 }} />}
                        label="JSON" 
                        size="small" 
                        variant="outlined"
                      />
                    </Box>
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

          <TablePagination
            component="div"
            count={-1} // 総数不明の場合は-1
            page={page}
            onPageChange={handleChangePage}
            rowsPerPage={rowsPerPage}
            onRowsPerPageChange={handleChangeRowsPerPage}
            rowsPerPageOptions={[25, 50, 100]}
            labelRowsPerPage="表示件数:"
            labelDisplayedRows={({ from, to }) => `${from}–${to}`}
          />
        </>
      )}

      {/* JSON表示ダイアログ */}
      <Dialog
        open={jsonDialogOpen}
        onClose={() => setJsonDialogOpen(false)}
        maxWidth="md"
        fullWidth
      >
        <DialogTitle>Event JSON</DialogTitle>
        <DialogContent>
          <Box
            component="pre"
            sx={{
              bgcolor: 'grey.100',
              p: 2,
              borderRadius: 1,
              overflow: 'auto',
              fontFamily: 'monospace',
              fontSize: '0.875rem',
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-all',
            }}
          >
            {selectedJson}
          </Box>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setJsonDialogOpen(false)}>閉じる</Button>
          <Button 
            onClick={() => {
              navigator.clipboard.writeText(selectedJson);
            }}
            variant="contained"
          >
            コピー
          </Button>
        </DialogActions>
      </Dialog>
    </Container>
  );
};
