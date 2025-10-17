import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  Container, Box, Typography, IconButton, Paper, List, ListItem, ListItemText,
  Chip, CircularProgress, Button, TextField, Select, MenuItem, FormControl,
  InputLabel, TablePagination, Dialog, DialogTitle, DialogContent, DialogActions,
  Tabs, Tab, Table, TableHead, TableBody, TableRow, TableCell, TableContainer,
  Tooltip
} from '@mui/material';
import { ArrowBack, ChatBubble, Person, AccessTime, Code, Psychology, Edit as EditIcon, History } from '@mui/icons-material';
import { useBots } from '../hooks/useBots';

interface BotReply {
  event_id: string;
  content: string;
  created_at: number;
  reply_to_event_id?: string;
  reply_to_user?: string;
  event_json: string;
}

interface UserImpression {
  id: number;
  bot_pubkey: string;
  user_pubkey: string;
  impression: string;
  created_at: number;
}

interface ImpressionsListResponse {
  impressions: UserImpression[];
  total: number;
  page: number;
  per_page: number;
}

export const BotDetailPage = () => {
  const { pubkey } = useParams<{ pubkey: string }>();
  const navigate = useNavigate();
  const { bots } = useBots();
  const [currentTab, setCurrentTab] = useState(0);
  
  // 返信履歴用の状態
  const [replies, setReplies] = useState<BotReply[]>([]);
  const [repliesLoading, setRepliesLoading] = useState(true);
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

  // 印象用の状態
  const [impressions, setImpressions] = useState<UserImpression[]>([]);
  const [impressionsLoading, setImpressionsLoading] = useState(false);
  const [impressionPage, setImpressionPage] = useState(0);
  const [impressionRowsPerPage, setImpressionRowsPerPage] = useState(20);
  const [totalImpressions, setTotalImpressions] = useState(0);
  const [editDialogOpen, setEditDialogOpen] = useState(false);
  const [historyDialogOpen, setHistoryDialogOpen] = useState(false);
  const [selectedUserPubkey, setSelectedUserPubkey] = useState<string>('');
  const [editingImpression, setEditingImpression] = useState<string>('');
  const [impressionHistory, setImpressionHistory] = useState<UserImpression[]>([]);

  const bot = bots.find(b => b.pubkey === pubkey);

  useEffect(() => {
    if (pubkey && currentTab === 0) {
      loadReplies();
    } else if (pubkey && currentTab === 1) {
      loadImpressions();
    }
  }, [pubkey, currentTab, page, rowsPerPage, actualSearchText, sortBy, sortOrder, impressionPage, impressionRowsPerPage]);

  // 返信履歴の読み込み
  const loadReplies = async () => {
    try {
      setRepliesLoading(true);
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
      setRepliesLoading(false);
      setInitialLoading(false);
    }
  };

  // 印象一覧の読み込み
  const loadImpressions = async () => {
    try {
      setImpressionsLoading(true);
      const params = new URLSearchParams({
        page: (impressionPage + 1).toString(),
        per_page: impressionRowsPerPage.toString(),
      });

      const response = await fetch(`/api/bots/${pubkey}/impressions?${params}`);
      if (response.ok) {
        const data: ImpressionsListResponse = await response.json();
        setImpressions(data.impressions);
        setTotalImpressions(data.total);
      }
    } catch (error) {
      console.error('印象一覧の取得エラー:', error);
    } finally {
      setImpressionsLoading(false);
    }
  };

  // 印象の変遷履歴を読み込み
  const loadImpressionHistory = async (userPubkey: string) => {
    try {
      const response = await fetch(`/api/bots/${pubkey}/impressions/${userPubkey}/history?per_page=50`);
      if (response.ok) {
        const data: UserImpression[] = await response.json();
        setImpressionHistory(data);
      }
    } catch (error) {
      console.error('印象履歴の取得エラー:', error);
    }
  };

  // 印象の編集を開く
  const handleEditImpression = (userPubkey: string, impression: string) => {
    setSelectedUserPubkey(userPubkey);
    setEditingImpression(impression);
    setEditDialogOpen(true);
  };

  // 印象の保存
  const handleSaveImpression = async () => {
    try {
      const response = await fetch(`/api/bots/${pubkey}/impressions/${selectedUserPubkey}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ impression: editingImpression }),
      });

      if (response.ok) {
        setEditDialogOpen(false);
        loadImpressions(); // 再読み込み
      }
    } catch (error) {
      console.error('印象の保存エラー:', error);
    }
  };

  // 履歴を表示
  const handleShowHistory = async (userPubkey: string) => {
    setSelectedUserPubkey(userPubkey);
    await loadImpressionHistory(userPubkey);
    setHistoryDialogOpen(true);
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

  const handleImpressionChangePage = (_event: unknown, newPage: number) => {
    setImpressionPage(newPage);
  };

  const handleImpressionChangeRowsPerPage = (event: React.ChangeEvent<HTMLInputElement>) => {
    setImpressionRowsPerPage(parseInt(event.target.value, 10));
    setImpressionPage(0);
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
              {bot.content ? JSON.parse(bot.content).name || 'Bot' : 'Bot'} 詳細
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ fontFamily: 'monospace' }}>
              {pubkey?.substring(0, 16)}...
            </Typography>
          </Box>
        </Box>
      </Box>

      {/* タブ */}
      <Paper sx={{ mb: 2 }}>
        <Tabs value={currentTab} onChange={(_, newValue) => setCurrentTab(newValue)}>
          <Tab label="返信履歴" />
          <Tab label="ユーザー印象" icon={<Psychology />} iconPosition="start" />
        </Tabs>
      </Paper>

      {/* 返信履歴タブ */}
      {currentTab === 0 && (
        <>
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
          {repliesLoading ? (
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
                count={-1}
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
        </>
      )}

      {/* ユーザー印象タブ */}
      {currentTab === 1 && (
        <>
          {impressionsLoading ? (
            <Box sx={{ display: 'flex', justifyContent: 'center', py: 4 }}>
              <CircularProgress size={40} />
            </Box>
          ) : impressions.length === 0 ? (
            <Paper sx={{ p: 4, textAlign: 'center' }}>
              <Typography color="text.secondary">
                まだ印象が記録されていません
              </Typography>
            </Paper>
          ) : (
            <>
              <TableContainer component={Paper}>
                <Table>
                  <TableHead>
                    <TableRow>
                      <TableCell>ユーザー</TableCell>
                      <TableCell>印象</TableCell>
                      <TableCell>更新日時</TableCell>
                      <TableCell align="right">操作</TableCell>
                    </TableRow>
                  </TableHead>
                  <TableBody>
                    {impressions.map((impression) => (
                      <TableRow key={impression.id} hover>
                        <TableCell sx={{ fontFamily: 'monospace', fontSize: '0.875rem' }}>
                          {impression.user_pubkey.substring(0, 16)}...
                        </TableCell>
                        <TableCell>
                          <Typography variant="body2" sx={{ 
                            maxWidth: 400, 
                            overflow: 'hidden', 
                            textOverflow: 'ellipsis',
                            whiteSpace: 'nowrap'
                          }}>
                            {impression.impression}
                          </Typography>
                        </TableCell>
                        <TableCell sx={{ fontFamily: 'monospace', fontSize: '0.875rem' }}>
                          {formatDate(impression.created_at)}
                        </TableCell>
                        <TableCell align="right">
                          <Tooltip title="履歴を表示">
                            <IconButton 
                              size="small" 
                              onClick={() => handleShowHistory(impression.user_pubkey)}
                            >
                              <History />
                            </IconButton>
                          </Tooltip>
                          <Tooltip title="編集">
                            <IconButton 
                              size="small" 
                              onClick={() => handleEditImpression(impression.user_pubkey, impression.impression)}
                            >
                              <EditIcon />
                            </IconButton>
                          </Tooltip>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </TableContainer>

              <TablePagination
                component="div"
                count={totalImpressions}
                page={impressionPage}
                onPageChange={handleImpressionChangePage}
                rowsPerPage={impressionRowsPerPage}
                onRowsPerPageChange={handleImpressionChangeRowsPerPage}
                rowsPerPageOptions={[10, 20, 50]}
                labelRowsPerPage="表示件数:"
                labelDisplayedRows={({ from, to, count }) => `${from}–${to} / ${count}`}
              />
            </>
          )}
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

      {/* 印象編集ダイアログ */}
      <Dialog
        open={editDialogOpen}
        onClose={() => setEditDialogOpen(false)}
        maxWidth="md"
        fullWidth
      >
        <DialogTitle>印象を編集</DialogTitle>
        <DialogContent>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2, fontFamily: 'monospace' }}>
            ユーザー: {selectedUserPubkey.substring(0, 16)}...
          </Typography>
          <TextField
            fullWidth
            multiline
            rows={8}
            value={editingImpression}
            onChange={(e) => setEditingImpression(e.target.value)}
            placeholder="このユーザーへの印象を入力..."
            inputProps={{ maxLength: 500 }}
            helperText={`${editingImpression.length} / 500文字`}
          />
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setEditDialogOpen(false)}>キャンセル</Button>
          <Button 
            onClick={handleSaveImpression}
            variant="contained"
            disabled={!editingImpression.trim()}
          >
            保存
          </Button>
        </DialogActions>
      </Dialog>

      {/* 印象履歴ダイアログ */}
      <Dialog
        open={historyDialogOpen}
        onClose={() => setHistoryDialogOpen(false)}
        maxWidth="md"
        fullWidth
      >
        <DialogTitle>印象の変遷履歴</DialogTitle>
        <DialogContent>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2, fontFamily: 'monospace' }}>
            ユーザー: {selectedUserPubkey.substring(0, 16)}...
          </Typography>
          {impressionHistory.length === 0 ? (
            <Typography color="text.secondary">履歴がありません</Typography>
          ) : (
            <List>
              {impressionHistory.map((history, index) => (
                <ListItem 
                  key={history.id}
                  sx={{ 
                    flexDirection: 'column', 
                    alignItems: 'flex-start',
                    borderLeft: index === 0 ? '3px solid' : '3px solid',
                    borderColor: index === 0 ? 'primary.main' : 'grey.300',
                    pl: 2,
                    mb: 2
                  }}
                >
                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1 }}>
                    {index === 0 && (
                      <Chip label="最新" size="small" color="primary" />
                    )}
                    <Typography variant="caption" color="text.secondary" sx={{ fontFamily: 'monospace' }}>
                      {formatDate(history.created_at)}
                    </Typography>
                  </Box>
                  <Typography variant="body2" sx={{ whiteSpace: 'pre-wrap' }}>
                    {history.impression}
                  </Typography>
                </ListItem>
              ))}
            </List>
          )}
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setHistoryDialogOpen(false)}>閉じる</Button>
        </DialogActions>
      </Dialog>
    </Container>
  );
};
