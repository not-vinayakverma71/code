// Service 1788
import { Injectable } from '@angular/core';
import { Observable } from 'rxjs';

@Injectable({ providedIn: 'root' })
export class Service1788 {
async getData(): Promise<any> {
const response = await fetch('/api/data');
return response.json();
}
}