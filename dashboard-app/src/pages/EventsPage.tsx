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
} from '@mui/material';
import { Search, FilterList, CheckCircle, Cancel } from '@mui/icons-material';
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
        
        {/* 統計情報 */}
        <Box sx={{ mb: 2 }}>
          <Typography variant="body2" color="text.secondary">
            全 {total.toLocaleString()} 件中 {events.length} 件表示
          </Typography>
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
                </TableRow>
              </TableHead>
              <TableBody>
                {loading && !initialLoading ? (
                  <TableRow>
                    <TableCell colSpan={9} align="center" sx={{ py: 4 }}>
                      <CircularProgress size={24} />
                    </TableCell>
                  </TableRow>
                ) : events.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={9} align="center" sx={{ py: 4 }}>
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
    </Container>
  );
};

