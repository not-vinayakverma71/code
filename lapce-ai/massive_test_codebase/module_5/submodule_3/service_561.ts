// Service 561
import { Injectable } from '@angular/core';
import { Observable } from 'rxjs';

@Injectable({ providedIn: 'root' })
export class Service561 {
async getData(): Promise<any> {
const response = await fetch('/api/data');
return response.json();
}
}