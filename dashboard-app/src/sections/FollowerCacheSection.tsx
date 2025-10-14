import { useState, useEffect } from 'react';
import { 
  Box, Typography, Paper, Button, Table, TableBody, TableCell, TableContainer, 
  TableHead, TableRow, Chip, IconButton, Tooltip 
} from '@mui/material';
import { People, Delete, Edit, DeleteSweep } from '@mui/icons-material';

interface FollowerCache {
  user_pubkey: string;
  user_name?: string;
  bot_pubkey: string;
  bot_name?: string;
  is_follower: boolean;
  cached_at: number;
}

export const FollowerCacheSection = () => {
  const [caches, setCaches] = useState<FollowerCache[]>([]);
  const [loading, setLoading] = useState(true);

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

  return (
    <Paper elevation={0} sx={{ p: 3, mb: 3, border: '1px solid', borderColor: 'divider', borderRadius: 2 }}>
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 3 }}>
        <Typography variant="h5" fontWeight="bold" sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <People /> フォロワーキャッシュ
          <Chip label={`${caches.length}件`} size="small" color="primary" sx={{ ml: 1 }} />
        </Typography>
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

      {loading ? (
        <Box sx={{ textAlign: 'center', py: 4 }}>読み込み中...</Box>
      ) : caches.length === 0 ? (
        <Box sx={{ textAlign: 'center', py: 4, color: 'text.secondary' }}>
          キャッシュがありません
        </Box>
      ) : (
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
              {caches.map((cache) => (
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
      )}
    </Paper>
  );
};

