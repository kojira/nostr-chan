import { useState, useEffect, useMemo } from 'react';
import { 
  Container, Box, Typography, IconButton, Paper, Button, Table, TableBody, 
  TableCell, TableContainer, TableHead, TableRow, Chip, Tooltip, TablePagination,
  TextField, InputAdornment, MenuItem, Select, FormControl, InputLabel
} from '@mui/material';
import { ArrowBack, Delete, DeleteSweep, People, Search, FilterList } from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';

interface FollowerCache {
  user_pubkey: string;
  user_name?: string;
  bot_pubkey: string;
  bot_name?: string;
  is_follower: boolean;
  cached_at: number;
}

export const FollowerCachePage = () => {
  const navigate = useNavigate();
  const [caches, setCaches] = useState<FollowerCache[]>([]);
  const [loading, setLoading] = useState(true);
  const [page, setPage] = useState(0);
  const [rowsPerPage, setRowsPerPage] = useState(25);
  const [userFilter, setUserFilter] = useState('');
  const [botFilter, setBotFilter] = useState('');
  const [followFilter, setFollowFilter] = useState<'all' | 'following' | 'not-following'>('all');

  const loadCaches = async () => {
    try {
      const response = await fetch('/api/follower-cache');
      if (!response.ok) throw new Error('取得に失敗しました');
      const data = await response.json();
      setCaches(data);
    } catch (error) {
      console.error('フォロワーキャッシュ取得エラー:', error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadCaches();
  }, []);

  const handleToggleFollower = async (userPubkey: string, botPubkey: string, currentStatus: boolean) => {
    try {
      const response = await fetch(`/api/follower-cache/${encodeURIComponent(userPubkey)}/${encodeURIComponent(botPubkey)}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ is_follower: !currentStatus }),
      });
      if (!response.ok) throw new Error('更新に失敗しました');
      loadCaches();
    } catch (error) {
      console.error('更新エラー:', error);
      alert('❌ 更新に失敗しました');
    }
  };

  const handleDelete = async (userPubkey: string, botPubkey: string) => {
    if (!confirm('このキャッシュを削除しますか？')) return;
    try {
      const response = await fetch(`/api/follower-cache/${encodeURIComponent(userPubkey)}/${encodeURIComponent(botPubkey)}`, {
        method: 'DELETE',
      });
      if (!response.ok) throw new Error('削除に失敗しました');
      loadCaches();
    } catch (error) {
      console.error('削除エラー:', error);
      alert('❌ 削除に失敗しました');
    }
  };

  const handleClearAll = async () => {
    if (!confirm('すべてのフォロワーキャッシュを削除しますか？')) return;
    try {
      const response = await fetch('/api/follower-cache', { method: 'DELETE' });
      if (!response.ok) throw new Error('削除に失敗しました');
      loadCaches();
      alert('✅ すべてのキャッシュを削除しました');
    } catch (error) {
      console.error('全削除エラー:', error);
      alert('❌ 削除に失敗しました');
    }
  };

  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString('ja-JP');
  };

  // フィルタ処理
  const filteredCaches = useMemo(() => {
    return caches.filter(cache => {
      // ユーザー名フィルタ
      if (userFilter) {
        const userName = cache.user_name?.toLowerCase() || '';
        const userPubkey = cache.user_pubkey.toLowerCase();
        const searchTerm = userFilter.toLowerCase();
        if (!userName.includes(searchTerm) && !userPubkey.includes(searchTerm)) {
          return false;
        }
      }

      // Bot名フィルタ
      if (botFilter) {
        const botName = cache.bot_name?.toLowerCase() || '';
        const botPubkey = cache.bot_pubkey.toLowerCase();
        const searchTerm = botFilter.toLowerCase();
        if (!botName.includes(searchTerm) && !botPubkey.includes(searchTerm)) {
          return false;
        }
      }

      // フォロー状態フィルタ
      if (followFilter === 'following' && !cache.is_follower) return false;
      if (followFilter === 'not-following' && cache.is_follower) return false;

      return true;
    });
  }, [caches, userFilter, botFilter, followFilter]);

  // ページネーション
  const paginatedCaches = filteredCaches.slice(page * rowsPerPage, page * rowsPerPage + rowsPerPage);

  // フィルタ変更時にページをリセット
  useEffect(() => {
    setPage(0);
  }, [userFilter, botFilter, followFilter]);

  return (
    <Container maxWidth="xl" sx={{ py: 4 }}>
      <Box sx={{ display: 'flex', alignItems: 'center', mb: 3 }}>
        <IconButton onClick={() => navigate('/')} sx={{ mr: 2 }}>
          <ArrowBack />
        </IconButton>
        <Typography variant="h4" fontWeight="bold">
          フォロワーキャッシュ管理
        </Typography>
      </Box>

      <Paper elevation={0} sx={{ p: 3, border: '1px solid', borderColor: 'divider', borderRadius: 2 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 3 }}>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
            <People sx={{ fontSize: 32 }} />
            <Typography variant="h6" fontWeight="bold">
              フォロワーキャッシュ一覧
            </Typography>
            <Chip label={`${filteredCaches.length} / ${caches.length}件`} size="small" color="primary" />
          </Box>
          <Button
            variant="outlined"
            color="error"
            startIcon={<DeleteSweep />}
            onClick={handleClearAll}
            disabled={caches.length === 0}
          >
            全削除
          </Button>
        </Box>

        {/* フィルタUI */}
        <Box sx={{ display: 'flex', gap: 2, mb: 3, flexWrap: 'wrap' }}>
          <TextField
            size="small"
            placeholder="ユーザー名またはPubkeyで検索..."
            value={userFilter}
            onChange={(e) => setUserFilter(e.target.value)}
            InputProps={{
              startAdornment: (
                <InputAdornment position="start">
                  <Search fontSize="small" />
                </InputAdornment>
              ),
            }}
            sx={{ minWidth: 300 }}
          />
          <TextField
            size="small"
            placeholder="Bot名またはPubkeyで検索..."
            value={botFilter}
            onChange={(e) => setBotFilter(e.target.value)}
            InputProps={{
              startAdornment: (
                <InputAdornment position="start">
                  <Search fontSize="small" />
                </InputAdornment>
              ),
            }}
            sx={{ minWidth: 300 }}
          />
          <FormControl size="small" sx={{ minWidth: 180 }}>
            <InputLabel>フォロー状態</InputLabel>
            <Select
              value={followFilter}
              onChange={(e) => setFollowFilter(e.target.value as typeof followFilter)}
              label="フォロー状態"
              startAdornment={
                <InputAdornment position="start">
                  <FilterList fontSize="small" />
                </InputAdornment>
              }
            >
              <MenuItem value="all">すべて</MenuItem>
              <MenuItem value="following">フォロー中のみ</MenuItem>
              <MenuItem value="not-following">未フォローのみ</MenuItem>
            </Select>
          </FormControl>
          {(userFilter || botFilter || followFilter !== 'all') && (
            <Button
              size="small"
              variant="outlined"
              onClick={() => {
                setUserFilter('');
                setBotFilter('');
                setFollowFilter('all');
              }}
            >
              フィルタクリア
            </Button>
          )}
        </Box>

        {loading ? (
          <Box sx={{ textAlign: 'center', py: 4 }}>読み込み中...</Box>
        ) : caches.length === 0 ? (
          <Box sx={{ textAlign: 'center', py: 4, color: 'text.secondary' }}>
            キャッシュがありません
          </Box>
        ) : (
          <>
            <TableContainer>
              <Table size="small">
                <TableHead>
                  <TableRow>
                    <TableCell><strong>ユーザー</strong></TableCell>
                    <TableCell><strong>Bot</strong></TableCell>
                    <TableCell align="center"><strong>フォロー状態</strong></TableCell>
                    <TableCell><strong>キャッシュ日時</strong></TableCell>
                    <TableCell align="center"><strong>操作</strong></TableCell>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {paginatedCaches.map((cache) => (
                    <TableRow key={`${cache.user_pubkey}-${cache.bot_pubkey}`}>
                      <TableCell>
                        {cache.user_name ? (
                          <Box>
                            <Typography variant="body2" fontWeight="bold">
                              {cache.user_name}
                            </Typography>
                            <Typography variant="caption" color="text.secondary" sx={{ fontFamily: 'monospace' }}>
                              {cache.user_pubkey.substring(0, 16)}...
                            </Typography>
                          </Box>
                        ) : (
                          <Typography variant="body2" sx={{ fontFamily: 'monospace', fontSize: '0.85rem' }}>
                            {cache.user_pubkey.substring(0, 16)}...
                          </Typography>
                        )}
                      </TableCell>
                      <TableCell>
                        {cache.bot_name ? (
                          <Box>
                            <Typography variant="body2" fontWeight="bold">
                              {cache.bot_name}
                            </Typography>
                            <Typography variant="caption" color="text.secondary" sx={{ fontFamily: 'monospace' }}>
                              {cache.bot_pubkey.substring(0, 16)}...
                            </Typography>
                          </Box>
                        ) : (
                          <Typography variant="body2" sx={{ fontFamily: 'monospace', fontSize: '0.85rem' }}>
                            {cache.bot_pubkey.substring(0, 16)}...
                          </Typography>
                        )}
                      </TableCell>
                      <TableCell align="center">
                        <Chip 
                          label={cache.is_follower ? 'フォロー中' : '未フォロー'}
                          size="small"
                          color={cache.is_follower ? 'success' : 'default'}
                          onClick={() => handleToggleFollower(cache.user_pubkey, cache.bot_pubkey, cache.is_follower)}
                          sx={{ cursor: 'pointer' }}
                        />
                      </TableCell>
                      <TableCell sx={{ fontSize: '0.85rem' }}>
                        {formatDate(cache.cached_at)}
                      </TableCell>
                      <TableCell align="center">
                        <Tooltip title="削除">
                          <IconButton 
                            size="small" 
                            color="error"
                            onClick={() => handleDelete(cache.user_pubkey, cache.bot_pubkey)}
                          >
                            <Delete fontSize="small" />
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
              count={filteredCaches.length}
              page={page}
              onPageChange={(_, newPage) => setPage(newPage)}
              rowsPerPage={rowsPerPage}
              onRowsPerPageChange={(e) => {
                setRowsPerPage(parseInt(e.target.value, 10));
                setPage(0);
              }}
              rowsPerPageOptions={[10, 25, 50, 100]}
              labelRowsPerPage="表示件数:"
              labelDisplayedRows={({ from, to, count }) => `${from}-${to} / ${count}件`}
            />
          </>
        )}
      </Paper>
    </Container>
  );
};

