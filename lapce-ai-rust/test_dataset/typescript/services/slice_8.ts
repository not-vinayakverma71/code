import { createSlice, PayloadAction } from '@reduxjs/toolkit';

interface State_8 {
    data: any[];
    loading: boolean;
    error: string | null;
}

const slice_8 = createSlice({
    name: 'feature_8',
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

export const { setData, setLoading } = slice_8.actions;
export default slice_8.reducer;