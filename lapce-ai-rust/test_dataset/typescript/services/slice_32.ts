import { createSlice, PayloadAction } from '@reduxjs/toolkit';

interface State_32 {
    data: any[];
    loading: boolean;
    error: string | null;
}

const slice_32 = createSlice({
    name: 'feature_32',
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

export const { setData, setLoading } = slice_32.actions;
export default slice_32.reducer;