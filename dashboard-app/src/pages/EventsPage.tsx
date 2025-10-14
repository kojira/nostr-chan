import { useState, useEffect } from 'react';
import {
  Container,
  Paper,
  Typography,
  Box,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TextField,
  Select,
  MenuItem,
  FormControl,
  InputLabel,
  Chip,
  Pagination,
  IconButton,
  Tooltip,
  CircularProgress,
  SelectChangeEvent,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  Snackbar,
  Alert,
} from '@mui/material';
import { Search, FilterList, CheckCircle, Cancel, Visibility, Delete, DeleteSweep } from '@mui/icons-material';
import { VectorizedEvent, EventsResponse } from '../types';

export const EventsPage = () => {
  const [events, setEvents] = useState<VectorizedEvent[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(50);
  const [totalPages, setTotalPages] = useState(0);
  const [loading, setLoading] = useState(false);
  const [initialLoading, setInitialLoading] = useState(true);
  
  // フィルター
  const [search, setSearch] = useState('');
  const [hasEmbedding, setHasEmbedding] = useState<string>('all');
  const [isJapanese, setIsJapanese] = useState<string>('all');
  const [eventType, setEventType] = useState<string>('');
  const [sortBy, setSortBy] = useState('created_at');
  const [sortOrder, setSortOrder] = useState('desc');
  
  // 日本語入力制御
  const [isComposing, setIsComposing] = useState(false);
  
  // JSON表示ダイアログ
  const [jsonDialogOpen, setJsonDialogOpen] = useState(false);
  const [selectedEventJson, setSelectedEventJson] = useState('');
  
  // スナックバー
  const [snackbarOpen, setSnackbarOpen] = useState(false);
  const [snackbarMessage, setSnackbarMessage] = useState('');
  const [snackbarSeverity, setSnackbarSeverity] = useState<'success' | 'error'>('success');

  const fetchEvents = async () => {
    if (initialLoading) {
      setLoading(true);
    }
    
    try {
      const params = new URLSearchParams({
        page: page.toString(),
        page_size: pageSize.toString(),
        sort_by: sortBy,
        sort_order: sortOrder,
      });
      
      if (search) params.append('search', search);
      if (hasEmbedding !== 'all') params.append('has_embedding', hasEmbedding);
      if (isJapanese !== 'all') params.append('is_japanese', isJapanese);
      if (eventType) params.append('event_type', eventType);
      
      const response = await fetch(`/api/events?${params}`);
      const data: EventsResponse = await response.json();
      
      setEvents(data.events);
      setTotal(data.total);
      setTotalPages(data.total_pages);
    } catch (error) {
      console.error('Failed to fetch events:', error);
    } finally {
      setLoading(false);
      setInitialLoading(false);
    }
  };

  useEffect(() => {
    if (!isComposing) {
      fetchEvents();
    }
  }, [page, pageSize, hasEmbedding, isJapanese, eventType, sortBy, sortOrder]);

  useEffect(() => {
    if (!isComposing && search !== undefined) {
      const timer = setTimeout(() => {
        setPage(1);
        fetchEvents();
      }, 500);
      return () => clearTimeout(timer);
    }
  }, [search, isComposing]);

  const formatTimestamp = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString('ja-JP');
  };

  const truncateText = (text: string, maxLength: number = 100) => {
    if (text.length <= maxLength) return text;
    return text.substring(0, maxLength) + '...';
  };

  const handleViewJson = async (event: VectorizedEvent) => {
    try {
      // イベントのJSONを取得（event_jsonフィールドがあればそれを使用、なければ構築）
      const eventJson = event.event_json || JSON.stringify(event, null, 2);
      setSelectedEventJson(eventJson);
      setJsonDialogOpen(true);
    } catch (error) {
      console.error('Failed to show JSON:', error);
      setSnackbarMessage('JSONの表示に失敗しました');
      setSnackbarSeverity('error');
      setSnackbarOpen(true);
    }
  };

  const handleDelete = async (eventId: number) => {
    if (!confirm('このイベントを削除しますか？')) return;
    
    try {
      const response = await fetch(`/api/events/${eventId}`, {
        method: 'DELETE',
      });
      
      if (response.ok) {
        setSnackbarMessage('イベントを削除しました');
        setSnackbarSeverity('success');
        setSnackbarOpen(true);
        fetchEvents();
      } else {
        throw new Error('削除に失敗しました');
      }
    } catch (error) {
      console.error('Failed to delete event:', error);
      setSnackbarMessage('削除に失敗しました');
      setSnackbarSeverity('error');
      setSnackbarOpen(true);
    }
  };

  const handleBulkDelete = async () => {
    const hasFilters = search || hasEmbedding !== 'all' || isJapanese !== 'all' || eventType;
    
    if (!hasFilters) {
      alert('フィルターを設定してください。全件削除を防ぐため、フィルターなしの一括削除はできません。');
      return;
    }
    
    const message = `現在のフィルター条件に一致する${total}件のイベントを削除しますか？この操作は取り消せません。`;
    if (!confirm(message)) return;
    
    try {
      const response = await fetch('/api/events/bulk-delete', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          search: search || undefined,
          has_embedding: hasEmbedding === 'all' ? undefined : hasEmbedding === 'true',
          is_japanese: isJapanese === 'all' ? undefined : isJapanese === 'true',
          event_type: eventType || undefined,
        }),
      });
      
      if (response.ok) {
        const data = await response.json();
        setSnackbarMessage(data.message || '一括削除しました');
        setSnackbarSeverity('success');
        setSnackbarOpen(true);
        fetchEvents();
      } else {
        throw new Error('一括削除に失敗しました');
      }
    } catch (error) {
      console.error('Failed to bulk delete:', error);
      setSnackbarMessage('一括削除に失敗しました');
      setSnackbarSeverity('error');
      setSnackbarOpen(true);
    }
  };

  return (
    <Container maxWidth="xl" sx={{ mt: 4, mb: 4 }}>
      <Paper sx={{ p: 3 }}>
        <Typography variant="h5" gutterBottom sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <FilterList />
          ベクトル化イベント一覧
        </Typography>
        
        {/* フィルター */}
        <Box sx={{ mb: 3, display: 'flex', gap: 2, flexWrap: 'wrap', alignItems: 'center' }}>
          <TextField
            label="検索"
            variant="outlined"
            size="small"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            onCompositionStart={() => setIsComposing(true)}
            onCompositionEnd={() => setIsComposing(false)}
            sx={{ minWidth: 300 }}
            InputProps={{
              startAdornment: <Search sx={{ mr: 1, color: 'text.secondary' }} />,
            }}
          />
          
          <FormControl size="small" sx={{ minWidth: 150 }}>
            <InputLabel>ベクトル化</InputLabel>
            <Select
              value={hasEmbedding}
              label="ベクトル化"
              onChange={(e: SelectChangeEvent) => {
                setHasEmbedding(e.target.value);
                setPage(1);
              }}
            >
              <MenuItem value="all">すべて</MenuItem>
              <MenuItem value="true">済</MenuItem>
              <MenuItem value="false">未</MenuItem>
            </Select>
          </FormControl>
          
          <FormControl size="small" sx={{ minWidth: 120 }}>
            <InputLabel>言語</InputLabel>
            <Select
              value={isJapanese}
              label="言語"
              onChange={(e: SelectChangeEvent) => {
                setIsJapanese(e.target.value);
                setPage(1);
              }}
            >
              <MenuItem value="all">すべて</MenuItem>
              <MenuItem value="true">日本語</MenuItem>
              <MenuItem value="false">その他</MenuItem>
            </Select>
          </FormControl>
          
          <TextField
            label="イベントタイプ"
            variant="outlined"
            size="small"
            value={eventType}
            onChange={(e) => {
              setEventType(e.target.value);
              setPage(1);
            }}
            sx={{ minWidth: 150 }}
          />
          
          <FormControl size="small" sx={{ minWidth: 120 }}>
            <InputLabel>並び順</InputLabel>
            <Select
              value={sortBy}
              label="並び順"
              onChange={(e: SelectChangeEvent) => setSortBy(e.target.value)}
            >
              <MenuItem value="created_at">作成日時</MenuItem>
              <MenuItem value="received_at">受信日時</MenuItem>
              <MenuItem value="content">内容</MenuItem>
              <MenuItem value="kind">Kind</MenuItem>
            </Select>
          </FormControl>
          
          <FormControl size="small" sx={{ minWidth: 100 }}>
            <InputLabel>順序</InputLabel>
            <Select
              value={sortOrder}
              label="順序"
              onChange={(e: SelectChangeEvent) => setSortOrder(e.target.value)}
            >
              <MenuItem value="desc">降順</MenuItem>
              <MenuItem value="asc">昇順</MenuItem>
            </Select>
          </FormControl>
          
          <FormControl size="small" sx={{ minWidth: 120 }}>
            <InputLabel>表示件数</InputLabel>
            <Select
              value={pageSize.toString()}
              label="表示件数"
              onChange={(e: SelectChangeEvent) => {
                setPageSize(Number(e.target.value));
                setPage(1);
              }}
            >
              <MenuItem value="10">10件</MenuItem>
              <MenuItem value="25">25件</MenuItem>
              <MenuItem value="50">50件</MenuItem>
              <MenuItem value="100">100件</MenuItem>
            </Select>
          </FormControl>
        </Box>
        
        {/* 統計情報と一括削除 */}
        <Box sx={{ mb: 2, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <Typography variant="body2" color="text.secondary">
            全 {total.toLocaleString()} 件中 {events.length} 件表示
          </Typography>
          <Button
            variant="outlined"
            color="error"
            startIcon={<DeleteSweep />}
            onClick={handleBulkDelete}
            disabled={total === 0}
          >
            フィルター条件で一括削除
          </Button>
        </Box>
        
        {/* テーブル */}
        {initialLoading ? (
          <Box sx={{ display: 'flex', justifyContent: 'center', py: 8 }}>
            <CircularProgress />
          </Box>
        ) : (
          <TableContainer>
            <Table size="small">
              <TableHead>
                <TableRow>
                  <TableCell>ID</TableCell>
                  <TableCell>Event ID</TableCell>
                  <TableCell>Kind</TableCell>
                  <TableCell>投稿者</TableCell>
                  <TableCell>内容</TableCell>
                  <TableCell>作成日時</TableCell>
                  <TableCell align="center">日本語</TableCell>
                  <TableCell align="center">ベクトル</TableCell>
                  <TableCell>タイプ</TableCell>
                  <TableCell align="center">操作</TableCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {loading && !initialLoading ? (
                  <TableRow>
                    <TableCell colSpan={10} align="center" sx={{ py: 4 }}>
                      <CircularProgress size={24} />
                    </TableCell>
                  </TableRow>
                ) : events.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={10} align="center" sx={{ py: 4 }}>
                      イベントが見つかりませんでした
                    </TableCell>
                  </TableRow>
                ) : (
                  events.map((event) => (
                    <TableRow key={event.id} hover>
                      <TableCell>{event.id}</TableCell>
                      <TableCell>
                        <Tooltip title={event.event_id}>
                          <Typography variant="body2" sx={{ fontFamily: 'monospace', fontSize: '0.75rem' }}>
                            {event.event_id.substring(0, 8)}...
                          </Typography>
                        </Tooltip>
                      </TableCell>
                      <TableCell>
                        <Chip label={event.kind} size="small" />
                      </TableCell>
                      <TableCell>
                        <Tooltip title={event.pubkey}>
                          <Box>
                            <Typography variant="body2">
                              {event.kind0_name || `${event.pubkey.substring(0, 8)}...`}
                            </Typography>
                          </Box>
                        </Tooltip>
                      </TableCell>
                      <TableCell>
                        <Tooltip title={event.content}>
                          <Typography variant="body2" sx={{ maxWidth: 300 }}>
                            {truncateText(event.content, 80)}
                          </Typography>
                        </Tooltip>
                      </TableCell>
                      <TableCell>
                        <Typography variant="body2" sx={{ fontSize: '0.75rem' }}>
                          {formatTimestamp(event.created_at)}
                        </Typography>
                      </TableCell>
                      <TableCell align="center">
                        {event.is_japanese ? (
                          <CheckCircle color="success" fontSize="small" />
                        ) : (
                          <Cancel color="disabled" fontSize="small" />
                        )}
                      </TableCell>
                      <TableCell align="center">
                        {event.has_embedding ? (
                          <CheckCircle color="primary" fontSize="small" />
                        ) : (
                          <Cancel color="disabled" fontSize="small" />
                        )}
                      </TableCell>
                      <TableCell>
                        {event.event_type && (
                          <Chip label={event.event_type} size="small" variant="outlined" />
                        )}
                      </TableCell>
                      <TableCell align="center">
                        <Tooltip title="JSON表示">
                          <IconButton size="small" onClick={() => handleViewJson(event)}>
                            <Visibility fontSize="small" />
                          </IconButton>
                        </Tooltip>
                        <Tooltip title="削除">
                          <IconButton size="small" color="error" onClick={() => handleDelete(event.id)}>
                            <Delete fontSize="small" />
                          </IconButton>
                        </Tooltip>
                      </TableCell>
                    </TableRow>
                  ))
                )}
              </TableBody>
            </Table>
          </TableContainer>
        )}
        
        {/* ページネーション */}
        {totalPages > 1 && (
          <Box sx={{ mt: 3, display: 'flex', justifyContent: 'center' }}>
            <Pagination
              count={totalPages}
              page={page}
              onChange={(_, value) => setPage(value)}
              color="primary"
              showFirstButton
              showLastButton
            />
          </Box>
        )}
      </Paper>

      {/* JSONダイアログ */}
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
              bgcolor: '#1e1e1e', 
              color: '#d4d4d4', 
              p: 2, 
              borderRadius: 1,
              overflow: 'auto',
              maxHeight: '60vh',
              fontSize: '0.875rem',
              fontFamily: 'monospace',
            }}
          >
            {selectedEventJson}
          </Box>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => {
            navigator.clipboard.writeText(selectedEventJson);
            setSnackbarMessage('JSONをコピーしました');
            setSnackbarSeverity('success');
            setSnackbarOpen(true);
          }}>
            コピー
          </Button>
          <Button onClick={() => setJsonDialogOpen(false)}>閉じる</Button>
        </DialogActions>
      </Dialog>

      {/* スナックバー */}
      <Snackbar
        open={snackbarOpen}
        autoHideDuration={3000}
        onClose={() => setSnackbarOpen(false)}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'center' }}
      >
        <Alert onClose={() => setSnackbarOpen(false)} severity={snackbarSeverity}>
          {snackbarMessage}
        </Alert>
      </Snackbar>
    </Container>
  );
};

