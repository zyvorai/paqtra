import { configureStore, createSlice, PayloadAction } from '@reduxjs/toolkit';

// Metrics slice
interface MetricsState {
  connected: boolean;
  lastUpdate: string | null;
}

const metricsSlice = createSlice({
  name: 'metrics',
  initialState: { connected: false, lastUpdate: null } as MetricsState,
  reducers: {
    setConnected(state, action: PayloadAction<boolean>) {
      state.connected = action.payload;
    },
    setLastUpdate(state, action: PayloadAction<string>) {
      state.lastUpdate = action.payload;
    },
  },
});

export const { setConnected, setLastUpdate } = metricsSlice.actions;

export const store = configureStore({
  reducer: {
    metrics: metricsSlice.reducer,
  },
});

export type RootState = ReturnType<typeof store.getState>;
export type AppDispatch = typeof store.dispatch;
