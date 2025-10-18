import { createSlice, PayloadAction } from '@reduxjs/toolkit';

interface State_16 {
    data: any[];
    loading: boolean;
    error: string | null;
}

const slice_16 = createSlice({
    name: 'feature_16',
    initialState: { data: [], loading: false, error: null },
    reducers: {
        setData: (state, action: PayloadAction<any[]>) => {
            state.data = action.payload;
        },
        setLoading: (state, action: PayloadAction<boolean>) => {
            state.loading = action.payload;
        },
    },
});

export const { setData, setLoading } = slice_16.actions;
export default slice_16.reducer;