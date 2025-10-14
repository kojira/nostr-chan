import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  Container, Box, Typography, IconButton, Paper, TextField, Select, MenuItem,
  FormControl, InputLabel, TablePagination, Button, Dialog, DialogTitle,
  DialogContent, DialogActions, List, ListItem, ListItemText, Chip, Snackbar, Alert,
  Avatar, Tooltip
} from '@mui/material';
import { ArrowBack, Summarize, Edit, Delete, Save, Cancel, AccessTime, DeleteSweep, Person } from '@mui/icons-material';
import { useBots } from '../hooks/useBots';

interface UserKind0 {
  pubkey: string;
  npub: string;
  name: string | null;
  display_name: string | null;
  picture: string | null;
  about: string | null;
  nip05: string | null;
}

interface Summary {
  id: number;
  bot_pubkey: string;
  summary: string;
  user_input: string;
  participants: string[] | null;
  from_timestamp: number;
  to_timestamp: number;
  created_at: number;
}

// 参加者アバターコンポーネント
const ParticipantAvatar = ({ pubkey, onClick }: { pubkey: string; onClick: () => void }) => {
  const [kind0, setKind0] = useState<UserKind0 | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchKind0 = async () => {
      try {
        const response = await fetch(`/api/users/${pubkey}/kind0`);
        if (response.ok) {
          const data = await response.json();
          console.log('Kind0データ:', pubkey.substring(0, 8), data);
          setKind0(data);
        }
      } catch (error) {
        console.error('Kind0取得エラー:', error);
      } finally {
        setLoading(false);
      }
    };
    fetchKind0();
  }, [pubkey]);

  if (loading) {
    return (
      <Avatar sx={{ width: 32, height: 32, cursor: 'pointer' }} onClick={onClick}>
        <Person fontSize="small" />
      </Avatar>
    );
  }

  const name = kind0?.display_name || kind0?.name || `${pubkey.substring(0, 8)}...`;
  const picture = kind0?.picture;

  return (
    <Tooltip title={name} arrow>
      <Avatar 
        src={picture || undefined}
        sx={{ width: 32, height: 32, cursor: 'pointer' }}
        onClick={onClick}
      >
        {!picture && <Person fontSize="small" />}
      </Avatar>
    </Tooltip>
  );
};

export const BotSummariesPage = () => {
  const { pubkey } = useParams<{ pubkey: string }>();
  const navigate = useNavigate();
  const { bots } = useBots();
  const [summaries, setSummaries] = useState<Summary[]>([]);
  const [loading, setLoading] = useState(true);
  const [initialLoading, setInitialLoading] = useState(true);
  const [page, setPage] = useState(0);
  const [rowsPerPage, setRowsPerPage] = useState(25);
  const [searchText, setSearchText] = useState('');
  const [actualSearchText, setActualSearchText] = useState('');
  const [isComposing, setIsComposing] = useState(false);
  const [sortBy, setSortBy] = useState<'created_at' | 'from_timestamp' | 'to_timestamp' | 'user_input'>('created_at');
  const [sortOrder, setSortOrder] = useState<'ASC' | 'DESC'>('DESC');
  
  // 編集ダイアログ
  const [editDialogOpen, setEditDialogOpen] = useState(false);
  const [editingSummary, setEditingSummary] = useState<Summary | null>(null);
  const [editFormData, setEditFormData] = useState({ summary: '', user_input: '' });
  
  // ユーザー情報ダイアログ
  const [userDialogOpen, setUserDialogOpen] = useState(false);
  const [selectedUserPubkey, setSelectedUserPubkey] = useState<string>('');
  const [selectedUserKind0, setSelectedUserKind0] = useState<UserKind0 | null>(null);
  
  // スナックバー
  const [snackbar, setSnackbar] = useState({
    open: false,
    message: '',
    severity: 'success' as 'success' | 'error' | 'info' | 'warning',
  });

  const bot = bots.find(b => b.pubkey === pubkey);

  useEffect(() => {
    if (pubkey) {
      loadSummaries();
    }
  }, [pubkey, page, rowsPerPage, actualSearchText, sortBy, sortOrder]);

  const loadSummaries = async () => {
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

      const response = await fetch(`/api/bots/${pubkey}/summaries?${params}`);
      if (response.ok) {
        const data: Summary[] = await response.json();
        setSummaries(data);
      }
    } catch (error) {
      console.error('要約取得エラー:', error);
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

  const handleEditClick = (summary: Summary) => {
    setEditingSummary(summary);
    setEditFormData({
      summary: summary.summary,
      user_input: summary.user_input,
    });
    setEditDialogOpen(true);
  };

  const handleEditSave = async () => {
    if (!editingSummary) return;
    
    try {
      const response = await fetch(`/api/summaries/${editingSummary.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(editFormData),
      });

      if (response.ok) {
        setEditDialogOpen(false);
        loadSummaries();
        setSnackbar({
          open: true,
          message: '要約を更新しました',
          severity: 'success',
        });
      } else {
        setSnackbar({
          open: true,
          message: '更新に失敗しました',
          severity: 'error',
        });
      }
    } catch (error) {
      console.error('更新エラー:', error);
      setSnackbar({
        open: true,
        message: '更新エラーが発生しました',
        severity: 'error',
      });
    }
  };

  const handleDelete = async (id: number) => {
    if (!confirm('この要約を削除しますか？')) return;
    
    try {
      const response = await fetch(`/api/summaries/${id}`, {
        method: 'DELETE',
      });

      if (response.ok) {
        loadSummaries();
        setSnackbar({
          open: true,
          message: '要約を削除しました',
          severity: 'success',
        });
      } else {
        setSnackbar({
          open: true,
          message: '削除に失敗しました',
          severity: 'error',
        });
      }
    } catch (error) {
      console.error('削除エラー:', error);
      setSnackbar({
        open: true,
        message: '削除エラーが発生しました',
        severity: 'error',
      });
    }
  };

  const handleBulkDelete = async () => {
    const message = actualSearchText
      ? `フィルタ条件に一致する要約を全て削除しますか？\n（検索: "${actualSearchText}"）`
      : 'このBotの要約を全て削除しますか？';
    
    if (!confirm(message)) return;
    
    try {
      const response = await fetch(`/api/bots/${pubkey}/summaries/bulk-delete`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ search: actualSearchText || null }),
      });

      if (response.ok) {
        const data = await response.json();
        loadSummaries();
        setSnackbar({
          open: true,
          message: `${data.deleted_count}件の要約を削除しました`,
          severity: 'success',
        });
      } else {
        setSnackbar({
          open: true,
          message: '一括削除に失敗しました',
          severity: 'error',
        });
      }
    } catch (error) {
      console.error('一括削除エラー:', error);
      setSnackbar({
        open: true,
        message: '一括削除エラーが発生しました',
        severity: 'error',
      });
    }
  };

  const handleUserClick = async (userPubkey: string) => {
    setSelectedUserPubkey(userPubkey);
    setUserDialogOpen(true);
    
    // Kind 0を取得
    try {
      const response = await fetch(`/api/users/${userPubkey}/kind0`);
      if (response.ok) {
        const data = await response.json();
        setSelectedUserKind0(data);
      }
    } catch (error) {
      console.error('Kind0取得エラー:', error);
    }
  };

  // バックエンドから取得したnpubを使用
  const getNpub = (): string => {
    return selectedUserKind0?.npub || selectedUserPubkey;
  };

  const handleCopyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    setSnackbar({
      open: true,
      message: 'コピーしました',
      severity: 'success',
    });
  };

  if (initialLoading) {
    return (
      <Container maxWidth="lg" sx={{ py: 4, display: 'flex', justifyContent: 'center' }}>
        <Typography>読み込み中...</Typography>
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
          <Summarize sx={{ fontSize: 32, color: 'primary.main' }} />
          <Box>
            <Typography variant="h4" fontWeight="bold">
              {bot.content ? JSON.parse(bot.content).name || 'Bot' : 'Bot'} の会話要約
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
            placeholder="要約・ユーザー入力で検索..."
          />
          <FormControl size="small" sx={{ minWidth: 150 }}>
            <InputLabel>ソート</InputLabel>
            <Select
              value={sortBy}
              label="ソート"
              onChange={(e) => {
                setSortBy(e.target.value as typeof sortBy);
                setPage(0);
              }}
            >
              <MenuItem value="created_at">作成日時</MenuItem>
              <MenuItem value="from_timestamp">期間開始</MenuItem>
              <MenuItem value="to_timestamp">期間終了</MenuItem>
              <MenuItem value="user_input">ユーザー入力</MenuItem>
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
          <Button
            variant="outlined"
            color="error"
            size="small"
            startIcon={<DeleteSweep />}
            onClick={handleBulkDelete}
            sx={{ minWidth: 150 }}
          >
            {actualSearchText ? 'フィルタ削除' : '全件削除'}
          </Button>
        </Box>
      </Paper>

      {/* 要約一覧 */}
      {loading ? (
        <Box sx={{ display: 'flex', justifyContent: 'center', py: 4 }}>
          <Typography>読み込み中...</Typography>
        </Box>
      ) : summaries.length === 0 ? (
        <Paper sx={{ p: 4, textAlign: 'center' }}>
          <Typography color="text.secondary">
            {actualSearchText ? '検索結果が見つかりません' : 'まだ要約がありません'}
          </Typography>
        </Paper>
      ) : (
        <>
          <List>
            {summaries.map((summary) => (
              <Paper key={summary.id} sx={{ mb: 2, p: 2 }}>
                <ListItem sx={{ flexDirection: 'column', alignItems: 'flex-start', p: 0 }}>
                  <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', width: '100%', mb: 1 }}>
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                      <AccessTime sx={{ fontSize: 16, color: 'text.secondary' }} />
                      <Typography variant="caption" color="text.secondary">
                        {formatDate(summary.from_timestamp)} 〜 {formatDate(summary.to_timestamp)}
                      </Typography>
                    </Box>
                    <Box sx={{ display: 'flex', gap: 1 }}>
                      <IconButton size="small" onClick={() => handleEditClick(summary)} color="primary">
                        <Edit fontSize="small" />
                      </IconButton>
                      <IconButton size="small" onClick={() => handleDelete(summary.id)} color="error">
                        <Delete fontSize="small" />
                      </IconButton>
                    </Box>
                  </Box>

                  <Box sx={{ width: '100%', mb: 2 }}>
                    <Typography variant="caption" color="text.secondary" fontWeight="bold">
                      ユーザー入力:
                    </Typography>
                    <Typography variant="body2" sx={{ mt: 0.5, p: 1, bgcolor: 'grey.50', borderRadius: 1 }}>
                      {summary.user_input}
                    </Typography>
                  </Box>

                  <Box sx={{ width: '100%', mb: 1 }}>
                    <Typography variant="caption" color="text.secondary" fontWeight="bold">
                      要約:
                    </Typography>
                    <ListItemText
                      primary={summary.summary}
                      primaryTypographyProps={{
                        sx: { whiteSpace: 'pre-wrap', wordBreak: 'break-word', mt: 0.5 }
                      }}
                    />
                  </Box>

                  {summary.participants && summary.participants.length > 0 && (
                    <Box sx={{ mt: 1, display: 'flex', alignItems: 'center', gap: 1 }}>
                      <Typography variant="caption" color="text.secondary" fontWeight="bold">
                        参加者:
                      </Typography>
                      <Box sx={{ display: 'flex', gap: 1 }}>
                        {summary.participants.map((p, idx) => (
                          <ParticipantAvatar key={idx} pubkey={p} onClick={() => handleUserClick(p)} />
                        ))}
                      </Box>
                    </Box>
                  )}
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

      {/* 編集ダイアログ */}
      <Dialog open={editDialogOpen} onClose={() => setEditDialogOpen(false)} maxWidth="md" fullWidth>
        <DialogTitle>要約編集</DialogTitle>
        <DialogContent>
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2, pt: 1 }}>
            <TextField
              label="ユーザー入力"
              multiline
              rows={3}
              value={editFormData.user_input}
              onChange={(e) => setEditFormData({ ...editFormData, user_input: e.target.value })}
              fullWidth
            />
            <TextField
              label="要約"
              multiline
              rows={8}
              value={editFormData.summary}
              onChange={(e) => setEditFormData({ ...editFormData, summary: e.target.value })}
              fullWidth
            />
          </Box>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setEditDialogOpen(false)} startIcon={<Cancel />}>
            キャンセル
          </Button>
          <Button onClick={handleEditSave} variant="contained" startIcon={<Save />}>
            保存
          </Button>
        </DialogActions>
      </Dialog>

      {/* ユーザー情報ダイアログ */}
      <Dialog open={userDialogOpen} onClose={() => setUserDialogOpen(false)} maxWidth="sm" fullWidth>
        <DialogTitle>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
            {selectedUserKind0?.picture && (
              <Avatar src={selectedUserKind0.picture} sx={{ width: 56, height: 56 }} />
            )}
            <Box>
              <Typography variant="h6">
                {selectedUserKind0?.display_name || selectedUserKind0?.name || 'ユーザー情報'}
              </Typography>
              {selectedUserKind0?.nip05 && (
                <Typography variant="caption" color="text.secondary">
                  {selectedUserKind0.nip05}
                </Typography>
              )}
            </Box>
          </Box>
        </DialogTitle>
        <DialogContent>
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
            {selectedUserKind0?.about && (
              <Box>
                <Typography variant="caption" color="text.secondary" fontWeight="bold">
                  自己紹介:
                </Typography>
                <Typography variant="body2" sx={{ mt: 0.5 }}>
                  {selectedUserKind0.about}
                </Typography>
              </Box>
            )}
            
            <Box>
              <Typography variant="caption" color="text.secondary" fontWeight="bold">
                npub:
              </Typography>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mt: 0.5 }}>
                <Typography 
                  variant="body2" 
                  sx={{ 
                    fontFamily: 'monospace',
                    wordBreak: 'break-all',
                    flex: 1,
                  }}
                >
                  {getNpub()}
                </Typography>
                <Button size="small" onClick={() => handleCopyToClipboard(getNpub())}>
                  コピー
                </Button>
              </Box>
            </Box>

            <Box>
              <Typography variant="caption" color="text.secondary" fontWeight="bold">
                Hex:
              </Typography>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mt: 0.5 }}>
                <Typography 
                  variant="body2" 
                  sx={{ 
                    fontFamily: 'monospace',
                    wordBreak: 'break-all',
                    flex: 1,
                  }}
                >
                  {selectedUserPubkey}
                </Typography>
                <Button size="small" onClick={() => handleCopyToClipboard(selectedUserPubkey)}>
                  コピー
                </Button>
              </Box>
            </Box>
          </Box>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setUserDialogOpen(false)}>閉じる</Button>
        </DialogActions>
      </Dialog>

      {/* スナックバー */}
      <Snackbar
        open={snackbar.open}
        autoHideDuration={4000}
        onClose={() => setSnackbar({ ...snackbar, open: false })}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'center' }}
      >
        <Alert
          onClose={() => setSnackbar({ ...snackbar, open: false })}
          severity={snackbar.severity}
          variant="filled"
          sx={{ width: '100%' }}
        >
          {snackbar.message}
        </Alert>
      </Snackbar>
    </Container>
  );
};

