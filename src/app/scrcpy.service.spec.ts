import { TestBed } from '@angular/core/testing';

import { ScrcpyService } from './scrcpy.service';

describe('ScrcpyService', () => {
  let service: ScrcpyService;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    service = TestBed.inject(ScrcpyService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });
});
