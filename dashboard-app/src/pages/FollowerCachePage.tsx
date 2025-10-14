import { useState, useEffect, useMemo } from 'react';
import { 
  Container, Box, Typography, IconButton, Paper, Button, Table, TableBody, 
  TableCell, TableContainer, TableHead, TableRow, Chip, Tooltip, TablePagination,
  TextField, InputAdornment, MenuItem, Select, FormControl, InputLabel, Dialog,
  DialogTitle, DialogContent, DialogActions
} from '@mui/material';
import { ArrowBack, Delete, DeleteSweep, People, Search, FilterList, ContentCopy, Settings, Save, Schedule } from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';

interface FollowerCache {
  user_pubkey: string;
  user_npub: string;
  user_name?: string;
  bot_pubkey: string;
  bot_npub: string;
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
  const [idDialogOpen, setIdDialogOpen] = useState(false);
  const [selectedId, setSelectedId] = useState<{ hex: string; npub: string; name?: string } | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [ttlSeconds, setTtlSeconds] = useState(86400);
  const [savingSettings, setSavingSettings] = useState(false);

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
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      const response = await fetch('/api/settings/follower-cache-ttl');
      if (response.ok) {
        const data = await response.json();
        setTtlSeconds(data.ttl_seconds);
      }
    } catch (error) {
      console.error('設定読み込みエラー:', error);
    }
  };

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

  const handleClearFiltered = async () => {
    if (filteredCaches.length === 0) {
      alert('削除対象がありません');
      return;
    }
    
    const message = `フィルタ中の ${filteredCaches.length} 件のキャッシュを削除しますか？`;
    if (!confirm(message)) return;

    try {
      let successCount = 0;
      let errorCount = 0;

      for (const cache of filteredCaches) {
        try {
          const response = await fetch(
            `/api/follower-cache/${encodeURIComponent(cache.user_pubkey)}/${encodeURIComponent(cache.bot_pubkey)}`,
            { method: 'DELETE' }
          );
          if (response.ok) {
            successCount++;
          } else {
            errorCount++;
          }
        } catch (error) {
          console.error('削除エラー:', error);
          errorCount++;
        }
      }

      loadCaches();
      
      if (errorCount === 0) {
        alert(`✅ ${successCount}件のキャッシュを削除しました`);
      } else {
        alert(`⚠️ ${successCount}件削除、${errorCount}件失敗しました`);
      }
    } catch (error) {
      console.error('フィルタ削除エラー:', error);
      alert('❌ 削除に失敗しました');
    }
  };

  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString('ja-JP');
  };

  const handleIdClick = (hex: string, npub: string, name?: string) => {
    setSelectedId({ hex, npub, name });
    setIdDialogOpen(true);
  };

  const handleCopyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    alert('クリップボードにコピーしました');
  };

  const handleSaveSettings = async () => {
    if (ttlSeconds < 60 || ttlSeconds > 604800) {
      alert('有効時間は60秒以上604800秒(7日間)以下で設定してください');
      return;
    }

    setSavingSettings(true);
    try {
      const response = await fetch('/api/settings/follower-cache-ttl', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ ttl_seconds: ttlSeconds }),
      });

      if (response.ok) {
        alert('✅ 設定を保存しました');
        setSettingsOpen(false);
      } else {
        alert('❌ 設定の保存に失敗しました');
      }
    } catch (error) {
      console.error('保存エラー:', error);
      alert('❌ 設定の保存に失敗しました');
    } finally {
      setSavingSettings(false);
    }
  };

  const getHoursDisplay = () => {
    const hours = ttlSeconds / 3600;
    if (hours >= 24) {
      return `${(hours / 24).toFixed(1)}日`;
    }
    return `${hours.toFixed(1)}時間`;
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
          <Box sx={{ display: 'flex', gap: 1 }}>
            <Button
              variant="outlined"
              startIcon={<Settings />}
              onClick={() => setSettingsOpen(true)}
            >
              設定
            </Button>
            {filteredCaches.length < caches.length && (
              <Button
                variant="outlined"
                color="warning"
                startIcon={<Delete />}
                onClick={handleClearFiltered}
                disabled={filteredCaches.length === 0}
              >
                フィルタ中を削除 ({filteredCaches.length})
              </Button>
            )}
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
                      <TableCell 
                        onClick={() => handleIdClick(cache.user_pubkey, cache.user_npub, cache.user_name)}
                        sx={{ cursor: 'pointer', '&:hover': { bgcolor: 'action.hover' } }}
                      >
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
                      <TableCell
                        onClick={() => handleIdClick(cache.bot_pubkey, cache.bot_npub, cache.bot_name)}
                        sx={{ cursor: 'pointer', '&:hover': { bgcolor: 'action.hover' } }}
                      >
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

      {/* IDダイアログ */}
      <Dialog open={idDialogOpen} onClose={() => setIdDialogOpen(false)} maxWidth="md" fullWidth>
        <DialogTitle>
          {selectedId?.name ? `${selectedId.name} の公開鍵` : '公開鍵'}
        </DialogTitle>
        <DialogContent>
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2, mt: 1 }}>
            {/* HEX形式 */}
            <Box>
              <Typography variant="subtitle2" color="text.secondary" gutterBottom>
                HEX形式
              </Typography>
              <Paper variant="outlined" sx={{ p: 2, bgcolor: 'grey.50', display: 'flex', alignItems: 'center', gap: 1 }}>
                <Typography variant="body2" sx={{ fontFamily: 'monospace', wordBreak: 'break-all', flex: 1 }}>
                  {selectedId?.hex}
                </Typography>
                <Tooltip title="コピー">
                  <IconButton 
                    size="small" 
                    onClick={() => selectedId?.hex && handleCopyToClipboard(selectedId.hex)}
                  >
                    <ContentCopy fontSize="small" />
                  </IconButton>
                </Tooltip>
              </Paper>
            </Box>

            {/* NPUB形式 */}
            <Box>
              <Typography variant="subtitle2" color="text.secondary" gutterBottom>
                NPUB形式
              </Typography>
              <Paper variant="outlined" sx={{ p: 2, bgcolor: 'grey.50', display: 'flex', alignItems: 'center', gap: 1 }}>
                <Typography variant="body2" sx={{ fontFamily: 'monospace', wordBreak: 'break-all', flex: 1 }}>
                  {selectedId?.npub}
                </Typography>
                <Tooltip title="コピー">
                  <IconButton 
                    size="small" 
                    onClick={() => selectedId?.npub && handleCopyToClipboard(selectedId.npub)}
                  >
                    <ContentCopy fontSize="small" />
                  </IconButton>
                </Tooltip>
              </Paper>
            </Box>
          </Box>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setIdDialogOpen(false)}>閉じる</Button>
        </DialogActions>
      </Dialog>

      {/* 設定ダイアログ */}
      <Dialog open={settingsOpen} onClose={() => setSettingsOpen(false)} maxWidth="sm" fullWidth>
        <DialogTitle>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
            <Settings />
            フォロワーキャッシュ設定
          </Box>
        </DialogTitle>
        <DialogContent>
          <Box sx={{ mt: 2 }}>
            <TextField
              label="キャッシュ有効時間（秒）"
              type="number"
              value={ttlSeconds}
              onChange={(e) => setTtlSeconds(parseInt(e.target.value) || 0)}
              fullWidth
              InputProps={{
                startAdornment: (
                  <InputAdornment position="start">
                    <Schedule />
                  </InputAdornment>
                ),
                endAdornment: (
                  <InputAdornment position="end">
                    <Typography variant="caption" color="text.secondary">
                      ≈ {getHoursDisplay()}
                    </Typography>
                  </InputAdornment>
                ),
              }}
              helperText="最小: 60秒 / 最大: 604800秒 (7日間)"
            />
            
            <Box sx={{ display: 'flex', gap: 1, mt: 2, flexWrap: 'wrap' }}>
              <Button variant="outlined" onClick={() => setTtlSeconds(3600)} size="small">
                1時間
              </Button>
              <Button variant="outlined" onClick={() => setTtlSeconds(21600)} size="small">
                6時間
              </Button>
              <Button variant="outlined" onClick={() => setTtlSeconds(86400)} size="small">
                24時間
              </Button>
              <Button variant="outlined" onClick={() => setTtlSeconds(604800)} size="small">
                7日間
              </Button>
            </Box>

            <Paper sx={{ mt: 2, p: 2, bgcolor: 'grey.50' }}>
              <Typography variant="caption" color="text.secondary">
                💡 フォロワーキャッシュはフォロワー判定の結果を一定時間保存します。
                長くすればリレーへの問い合わせが減りますが、フォロー状態の変更が反映されるまで時間がかかります。
              </Typography>
            </Paper>
          </Box>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setSettingsOpen(false)}>
            キャンセル
          </Button>
          <Button 
            onClick={handleSaveSettings} 
            variant="contained" 
            startIcon={<Save />}
            disabled={savingSettings || ttlSeconds < 60 || ttlSeconds > 604800}
          >
            {savingSettings ? '保存中...' : '保存'}
          </Button>
        </DialogActions>
      </Dialog>
    </Container>
  );
};

