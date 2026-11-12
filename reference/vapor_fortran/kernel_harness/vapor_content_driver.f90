program vapor_content_driver
  implicit none
  integer :: nreq, r, n, nmat, i, j, nrecords, ios
  integer, allocatable :: material(:)
  real, allocatable :: theta(:), theta_v(:), temp(:), hnew(:), cap(:), ths(:)
  real :: xconv
  character(len=128) :: request_id

  read(*, *, iostat=ios) nreq
  if (ios /= 0 .or. nreq < 1) error stop 10
  nrecords = 0
  do r = 1, nreq
    read(*, *, iostat=ios) request_id, n, nmat, xconv
    if (ios /= 0 .or. n < 1 .or. nmat < 1) error stop 11
    allocate(theta(n), theta_v(n), temp(n), hnew(n), cap(n), material(n), ths(nmat))
    do j = 1, nmat
      read(*, *, iostat=ios) ths(j)
      if (ios /= 0) error stop 12
    end do
    do i = 1, n
      read(*, *, iostat=ios) theta(i), temp(i), hnew(i), material(i), cap(i)
      if (ios /= 0) error stop 13
    end do
    call VaporContent(n, nmat, theta, theta_v, temp, hnew, material, ths, cap, xconv)
    do i = 1, n
      write(*, '(A,1X,A,1X,I0,1X,ES24.16E3,1X,ES24.16E3)') &
        'RECORD', trim(request_id), i, theta_v(i), cap(i)
      nrecords = nrecords + 1
    end do
    deallocate(theta, theta_v, temp, hnew, cap, material, ths)
  end do
  write(*, '(A,1X,I0,1X,I0)') 'COMPLETE', nreq, nrecords
end program vapor_content_driver
